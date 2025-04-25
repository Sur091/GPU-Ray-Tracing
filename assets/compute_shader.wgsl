// Compute shader for GPU ray tracing

// Texture bindings for input/output
@group(0) @binding(0) var input: texture_storage_2d<rgba32float, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba32float, write>;

struct SceneCamera {
    center: vec3<f32>,
    viewport_height: f32,   // No uses

    viewport_upper_left: vec3<f32>,
    viewport_width: f32,     // No uses

    pixel_delta_u: vec3<f32>,
    defocus_angle: f32,

    pixel_delta_v: vec3<f32>,
    aspect_ratio: f32,       // No uses

    defocus_disk_u: vec3<f32>,
    _padding0: f32,

    viewport_u: vec3<f32>,   // No uses
    _padding1: f32,

    defocus_disk_v: vec3<f32>,
    max_depth: f32,

    look_from: vec3<f32>,  // No uses
    samples_per_pixel: f32,

    look_at: vec3<f32>,    // No uses
    camera_has_moved: f32,

    vup: vec3<f32>,   // No uses
    random_seed: f32,

    viewport_v: vec3<f32>,  // No uses
    defocus_radius: f32     // No uses
}

@group(1) @binding(0) var<uniform> camera: SceneCamera;


// Sphere data
@group(2) @binding(0) var<uniform> sphere_count: u32;
@group(2) @binding(1) var<storage, read_write> spheres: array<Sphere>;

// Random number utilities
fn hash(value: u32) -> u32 {
    var state = value;
    state = state ^ 2747636419u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    return state;
}

fn random_float(value: u32) -> f32 {
    return f32(hash(value)) / 4294967295.0;
}

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Initialize the texture
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    textureStore(output, location, vec4<f32>(0.0));
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>
}

// material.albedo.z < -1.0 means the material is lambertian
// material.albedo.z between -1.0 and 1.0 means the material is metallic
// material.albedo.z > 1.0 means the material is refractive
struct Material {
    albedo: vec4<f32>,
}

fn lambertian_scatter(material: Material, ray: Ray, hit_record: HitRecord, attenuation: ptr<function, vec3<f32>>, scattered: ptr<function, Ray>, seed: u32) -> bool {
    var scattered_direction = hit_record.normal + random_unit_vector(seed);
    // Ignore zero-length vectors
    if (dot(scattered_direction, scattered_direction) < 1e-6) {
        scattered_direction = hit_record.normal;
    }
    *scattered =  Ray(hit_record.p, scattered_direction);
    *attenuation = material.albedo.xyz;
    return true;
}

fn metal_scatter(material: Material, ray: Ray, hit_record: HitRecord, attenuation: ptr<function, vec3<f32>>, scattered: ptr<function, Ray>, seed: u32) -> bool {
    let reflected = normalize(reflect(ray.direction, hit_record.normal)) + material.albedo.w * random_unit_vector(seed);
    *scattered = Ray(hit_record.p, normalize(reflected));
    *attenuation = material.albedo.xyz;
    return dot(reflected, hit_record.normal) > 0.0;
}

fn dielectric_scatter(material: Material, ray: Ray, hit_record: HitRecord, attenuation: ptr<function, vec3<f32>>, scattered: ptr<function, Ray>, seed: u32) -> bool {
    // Dielectric materials don't absorb light, so attenuation is 1.0
    *attenuation = vec3<f32>(1.0);

    // Calculate the refraction ratio based on whether we're entering or exiting the material
    // When front_face is true, we're entering the material (air->glass)
    // When front_face is false, we're exiting the material (glass->air)
    let refraction_ratio = select(material.albedo.x, 1.0 / material.albedo.x, hit_record.front_face);

    // Ensure ray direction is normalized
    let unit_direction = normalize(ray.direction);

    // Calculate the cosine of angle between incoming ray and normal
    let cos_theta = min(dot(-unit_direction, hit_record.normal), 1.0);
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);

    // Check for total internal reflection
    let cannot_refract = refraction_ratio * sin_theta > 1.0;
    let should_reflect = cannot_refract || reflectance(cos_theta, refraction_ratio) > random_float(seed);

    // Calculate the scattered ray direction
    let direction = select(
        refract(unit_direction, hit_record.normal, refraction_ratio),  // Refract
        reflect(unit_direction, hit_record.normal),                     // Reflect
        should_reflect
    );



    // Set the scattered ray with an appropriate offset to avoid self-intersection
    *scattered = Ray(hit_record.p , normalize(direction));

    return true;
}

fn reflectance(cos_theta: f32, refractive_index: f32) -> f32 {
    var r0 = (1.0 - refractive_index) / (1.0 + refractive_index);
    r0 *= r0;
    return r0 + (1.0 - r0) * pow(1.0 - cos_theta, 5.0);
}

struct HitRecord {
    t: f32,
    p: vec3<f32>,
    normal: vec3<f32>,
    front_face: bool,
    material: Material
}

struct Sphere {
    center: vec3<f32>,
    radius: f32,
    material: Material
}


fn hit_record_set_face_normal(rec: ptr<function, HitRecord>, r: Ray, outward_normal: vec3<f32>) {
    let front_face = dot(r.direction, outward_normal) < 0.0;
    let normal = select(-outward_normal, outward_normal, front_face);
    *rec = HitRecord((*rec).t, (*rec).p, normal, front_face, (*rec).material);
}

fn sphere_list_hit(r: Ray, ray_tmin: f32, ray_tmax: f32, rec: ptr<function, HitRecord>) -> bool {
    var temp_rec = HitRecord(0.0, vec3<f32>(0.0), vec3<f32>(0.0), false, Material(vec4<f32>(0.0)));
    var hit_anything = false;
    var closest_so_far = ray_tmax;

    for (var i: u32 = 0u; i < sphere_count; i++) {
        let sphere = spheres[i];  // Dereference the pointer here
        let hit = sphere_hit(sphere, r, ray_tmin, closest_so_far, &temp_rec);
        if hit {
            hit_anything = true;
            closest_so_far = temp_rec.t;
            *rec = temp_rec;
        }
    }

    return hit_anything;
}

fn sphere_hit(sphere: Sphere, r: Ray, ray_tmin: f32, ray_tmax: f32, rec: ptr<function, HitRecord>) -> bool {
    let oc = sphere.center - r.origin;
    let a = dot(r.direction, r.direction);
    let h = dot(oc, r.direction);
    let c = dot(oc, oc) - sphere.radius * sphere.radius;
    let discriminant = h*h - a*c;

    if discriminant < 0.0 {
        return false;
    }

    let square_root = sqrt(discriminant);

    var root = (h - square_root) / a;
    if root <= ray_tmin || ray_tmax <= root {
        root = (h + square_root) / a;
        if root <= ray_tmin || ray_tmax <= root {
            return false;
        }
    }


    // Calculate hit point
    let hit_point = r.origin + root * r.direction;
    let outward_normal = (hit_point - sphere.center) / sphere.radius;

    // Create new HitRecord with updated values
    *rec = HitRecord(
        root,
        hit_point,
        outward_normal,
        false,
        sphere.material
    );

    // Set the face normal
    hit_record_set_face_normal(rec, r, outward_normal);

    return true;
}

// Generate a random vec3 with components in [0,1]
fn random_vec3(seed: u32) -> vec3<f32> {
    // Use different derived seeds for each component
    return vec3<f32>(
        random_float(seed),
        random_float(seed + 1u),
        random_float(seed + 2u)
    );
}

// Generate a random unit vector (a point on the unit sphere)
fn random_unit_vector(seed: u32) -> vec3<f32> {
    // Generate 3 random components
    let z = 2.0 * random_float(seed) - 1.0;
    let a = random_float(seed + 1u) * 6.283185307;
    let r = sqrt(1.0 - z*z);
    let x = r * cos(a);
    let y = r * sin(a);

    return vec3<f32>(x, y, z);
}

// Generate a random unit vector in hemisphere around normal
fn random_in_hemisphere(normal: vec3<f32>, seed: u32) -> vec3<f32> {
    // Get random unit vector
    let unit_vector = random_unit_vector(seed);

    // Check if it's in the same hemisphere as the normal
    // (dot product > 0 means the angle is less than 90 degrees)
    if dot(unit_vector, normal) > 0.0 {
        // Already in correct hemisphere
        return unit_vector;
    } else {
        // In opposite hemisphere, so flip it
        return -unit_vector;
    }
}

fn ray_color(ray: Ray, seed: u32) -> vec3<f32> {
    var r = ray;
    var color_factor = vec3<f32>(1.0);
    for (var i: u32 = 0; i < u32(camera.max_depth); i++) {
        var hit_record = HitRecord(0.0, vec3<f32>(0.0), vec3<f32>(0.0), false, Material(vec4<f32>(0.0)));
        let t = sphere_list_hit(r, 0.001, 3.4e35, &hit_record);
        if t {
            let seed = hash(seed + i * 1000u);
            var scattered = Ray(vec3<f32>(0.0), vec3<f32>(0.0));
            var attenuation = vec3<f32>(0.0);
            // If material is lambertian
            if (hit_record.material.albedo.w < -1.0) {
                if (!lambertian_scatter(hit_record.material, r, hit_record, &attenuation, &scattered, seed)) {
                    return vec3<f32>(0.0);
                }
            } else if (hit_record.material.albedo.w <= 1.0) {
                if (!metal_scatter(hit_record.material, r, hit_record, &attenuation, &scattered, seed)) {
                    return vec3<f32>(0.0);
                }
            } else {
                if (!dielectric_scatter(hit_record.material, r, hit_record, &attenuation, &scattered, seed)) {
                    return vec3<f32>(0.0);
                }
            }
            color_factor *= attenuation;
            r = scattered;
        }
        else {
            break;
        }
    }

    let unit_direction = normalize(r.direction);
    let a = 0.5*(unit_direction.y + 1.0);
    let sky_color = (1.0-a)*vec3<f32>(1.0, 1.0, 1.0) + a*vec3<f32>(0.5, 0.7, 1.0);
    return color_factor * sky_color;
}

fn sample_square(seed: u32) -> vec3<f32> {
    let x = random_float(seed) - 0.5;
    let y = random_float(seed * seed) - 0.5;
    return vec3<f32>(x, y, 0.0);
}

fn get_ray(
    location: vec2<i32>,
    sample_index: u32
) -> Ray {
    let seed = hash(hash(u32(location.x) * 73u) ^
               (hash(u32(location.y) * 51u)) ^
               (sample_index * 25u + u32(camera.random_seed * 4294967295.0)));
    let offset = sample_square(seed);

    // Calculate pixel center
    let pixel_center = camera.viewport_upper_left
        + camera.pixel_delta_u * (f32(location.x) + 0.5 + offset.x)
        + camera.pixel_delta_v * (f32(location.y) + 0.5 + offset.y);

    let ray_origin = select(camera.center, defocus_disk_sample(seed+1u), camera.defocus_angle > 0.0);
    // let ray_origin = defocus_disk_sample(seed+30u);

    let ray_direction = pixel_center - ray_origin;

    return Ray(ray_origin, ray_direction);
}

fn defocus_disk_sample(seed: u32) -> vec3<f32> {
    let angle = 2.0 * 3.1415926 * random_float(seed);
    let p = normalize(vec2<f32>(cos(angle), sin(angle)));
    return camera.center + (p.x * camera.defocus_disk_u) + (p.y * camera.defocus_disk_v);
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let size = textureDimensions(output);


    let progress = textureLoad(input, location);
    var color_until_now = progress.xyz;
    var samples_until_now: u32 = u32(progress.w);

    let samples_per_pixel = u32(camera.samples_per_pixel);

    let reset = camera.camera_has_moved > 0.5;

    if (reset) {
        color_until_now = vec3<f32>(0.0);
        samples_until_now = 0u;
    }

    if (samples_until_now < samples_per_pixel) {
        let seed = 1u + samples_until_now + u32(camera.random_seed * 4294967295.0);
        let ray = get_ray(location, seed);
        let color = ray_color(ray, seed+1u);
        color_until_now += (color - color_until_now) / f32(samples_until_now + 1u);
        samples_until_now += 1u;
    }



    let final_color = vec4<f32>(color_until_now, f32(samples_until_now));
    textureStore(output, location, final_color);
}
