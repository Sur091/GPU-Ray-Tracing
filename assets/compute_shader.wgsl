// Compute shader for GPU ray tracing

// Texture bindings for input/output
@group(0) @binding(0) var input: texture_storage_2d<rgba32float, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba32float, write>;

// Camera uniform buffer
struct SceneCamera {
    position: vec3<f32>,
    focal_length: f32,
    view_direction: vec3<f32>,
    field_of_view: f32,
    reset_seed_depth_samples: vec4<f32>, // x: reset flag, y: random_seed, z: MAX_DEPTH, w: samples
}
@group(1) @binding(0) var<uniform> camera: SceneCamera;

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

const NUMBER_OF_SPHERES = 5;
struct SphereList {
    spheres: array<Sphere, NUMBER_OF_SPHERES>
}

fn hit_record_set_face_normal(rec: ptr<function, HitRecord>, r: Ray, outward_normal: vec3<f32>) {
    let front_face = dot(r.direction, outward_normal) < 0.0;
    let normal = select(-outward_normal, outward_normal, front_face);
    *rec = HitRecord((*rec).t, (*rec).p, normal, front_face, (*rec).material);
}

fn sphere_list_hit(sphere_list: ptr<function, SphereList>, r: Ray, ray_tmin: f32, ray_tmax: f32, rec: ptr<function, HitRecord>) -> bool {
    var temp_rec = HitRecord(0.0, vec3<f32>(0.0), vec3<f32>(0.0), false, Material(vec4<f32>(0.0)));
    var hit_anything = false;
    var closest_so_far = ray_tmax;

    for (var i = 0; i < NUMBER_OF_SPHERES; i++) {
        let sphere = (*sphere_list).spheres[i];  // Dereference the pointer here
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

fn ray_color(ray: Ray, sphere_list: ptr<function, SphereList>, seed: u32) -> vec3<f32> {
    var r = ray;
    var color_factor = vec3<f32>(1.0);
    for (var i: u32 = 0; i < u32(camera.reset_seed_depth_samples.z); i++) {
        var hit_record = HitRecord(0.0, vec3<f32>(0.0), vec3<f32>(0.0), false, Material(vec4<f32>(0.0)));
        let t = sphere_list_hit(sphere_list, r, 0.001, 3.4e35, &hit_record);
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

fn get_ray(location: vec2<i32>, viewport_upper_left: vec3<f32>, pixel_delta_u: vec3<f32>, pixel_delta_v: vec3<f32>, camera_center: vec3<f32>, sample_index: u32) -> Ray {
    let seed = hash(hash(u32(location.x) * 73u) ^
               (hash(u32(location.y) * 51u)) ^
               (sample_index * 25u + u32(camera.reset_seed_depth_samples.y * 4294967295.0)));
    let offset = sample_square(seed);

    // Calculate pixel center
    let pixel_center = viewport_upper_left
        + pixel_delta_u * (f32(location.x) + 0.5 + offset.x)
        + pixel_delta_v * (f32(location.y) + 0.5 + offset.y);

    let ray_direction = pixel_center - camera_center;

    return Ray(camera_center, ray_direction);
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let size = textureDimensions(output);
    let aspect_ratio = f32(size.x) / f32(size.y);

    // Use camera data from uniform buffer
    let focal_length = camera.focal_length;
    let h = tan(radians(camera.field_of_view / 2.0));
    let viewport_height = 2.0 * h * focal_length;
    let viewport_width = viewport_height * aspect_ratio;
    let camera_center = camera.position;

    // Calculate viewport vectors
    // Use view direction to calculate viewport orientation
    let w = normalize(-camera.view_direction);
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let u = normalize(cross(up, w));
    let v = cross(w, u);

    let viewport_u = viewport_width * u;
    let viewport_v = -viewport_height * v; // Negative to flip y-axis

    // Calculate pixel deltas
    let pixel_delta_u = viewport_u / f32(size.x);
    let pixel_delta_v = viewport_v / f32(size.y);

    // Calculate viewport upper left corner
    let viewport_upper_left = camera_center
        - viewport_u / 2.0
        - viewport_v / 2.0
        - focal_length * w;



    // Check the progress
    let progress = textureLoad(input, location);
    var color_until_now = progress.xyz;
    var samples_until_now = u32(progress.w);


    let material_ground = Material(vec4<f32>(0.8, 0.8, 0.0, -2.0));
    let material_center = Material(vec4<f32>(0.1, 0.2, 0.5, -2.0));
    // Glass sphere with refractive index 1.5 (typical for glass)
    let material_left   = Material(vec4<f32>(1.5, 0.0, 0.0, 2.0));
    // Hollow sphere (air bubble) with refractive index 1.0/1.5
    let material_bubble = Material(vec4<f32>(1.0/1.5, 0.0, 0.0, 2.0));
    let material_right  = Material(vec4<f32>(0.8, 0.6, 0.2, 1.0));

    let sphere2 = Sphere(vec3<f32>(0.0, -100.5, -1.0), 100.0, material_ground);
    let sphere1 = Sphere(vec3<f32>(0.0, 0.0, -1.2), 0.5, material_center);
    let sphere3 = Sphere(vec3<f32>(-1.0, 0.0, -1.0), 0.5, material_left);
    let sphere4 = Sphere(vec3<f32>(-1.0, 0.0, -1.0), 0.4, material_bubble);
    let sphere5 = Sphere(vec3<f32>(1.0, 0.0, -1.0), 0.5, material_right);

    var sphere_list = SphereList(array<Sphere, NUMBER_OF_SPHERES>(sphere1, sphere2, sphere3, sphere4, sphere5));

    let samples_per_pixel = u32(camera.reset_seed_depth_samples.w);
    
    let reset = camera.reset_seed_depth_samples.x > 0.5;
    
    if (reset) {
        color_until_now = vec3<f32>(0.0);
        samples_until_now = 1u;
    }

    if (samples_until_now < samples_per_pixel) {
        let seed = 1u + samples_until_now + u32(camera.reset_seed_depth_samples.y * 4294967295.0);
        let ray = get_ray(location, viewport_upper_left, pixel_delta_u, pixel_delta_v, camera_center, seed);
        let color = ray_color(ray, &sphere_list, seed+1u);
        color_until_now += (color - color_until_now) / f32(samples_until_now + 1u);
        samples_until_now += 1u;
    }

    let final_color = vec4<f32>(color_until_now, f32(samples_until_now));
    textureStore(output, location, final_color);
}
