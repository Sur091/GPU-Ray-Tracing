// gpu_ray_tracing/assets/compute_shader_example.wgsl

// Use rgba32float for accumulation
@group(0) @binding(0) var input: texture_storage_2d<rgba32float, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba32float, write>;


// Updated Camera Uniform Struct
struct CameraUniform {
    position: vec3<f32>,
    focal_length: f32,
    view_direction: vec3<f32>,
    viewport_height: f32,
    // Combined field for frame count, reset flag, samples per pixel
    frame_count_reset_samples: vec4<f32>, // x: frame_count, y: reset_flag (1.0=reset), z: unused, w: samples_per_pixel
}


@group(1) @binding(0) var<uniform> camera: CameraUniform;


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

fn randomFloat(value: u32) -> f32 {
    return f32(hash(value)) / 4294967295.0;
}




struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>
}

// material.albedo.w < -1.0 means the material is lambertian
// material.albedo.w between -1.0 and 1.0 means the material is metallic
// material.albedo.w > 1.0 means the material is refractive
struct Material {
    albedo: vec4<f32>,
}

fn lambertian_scatter(material: Material, ray: Ray, hit_record: HitRecord, attenuation: ptr<function, vec3<f32>>, scattered: ptr<function, Ray>, seed: u32) -> bool {
    var scattered_direction = hit_record.normal + random_unit_vector(seed);
    // Ignore zero-length vectors
    if (dot(scattered_direction, scattered_direction) < 1e-6) {
        scattered_direction = hit_record.normal;
    }
    *scattered =  Ray(hit_record.p + hit_record.normal * 0.001, scattered_direction);
    *attenuation = material.albedo.xyz;
    return true;
}

fn metal_scatter(material: Material, ray: Ray, hit_record: HitRecord, attenuation: ptr<function, vec3<f32>>, scattered: ptr<function, Ray>, seed: u32) -> bool {
    let reflected = normalize(reflect(normalize(ray.direction), hit_record.normal)) + material.albedo.w * random_unit_vector(seed);
    *scattered = Ray(hit_record.p + hit_record.normal * 0.001, normalize(reflected));
    *attenuation = material.albedo.xyz;
    return dot(reflected, hit_record.normal) > 0.0;
}

fn dielectric_scatter(material: Material, ray: Ray, hit_record: HitRecord, attenuation: ptr<function, vec3<f32>>, scattered: ptr<function, Ray>, seed: u32) -> bool {
    *attenuation = vec3<f32>(1.0);
    let refraction_ratio = select(material.albedo.x, 1.0 / material.albedo.x, hit_record.front_face);

    let unit_direction = normalize(ray.direction);
    let cos_theta = min(dot(-unit_direction, hit_record.normal), 1.0);
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);

    // Check for total internal reflection
    let cannot_refract = refraction_ratio * sin_theta > 1.0;

    var direction: vec3<f32>;

    // Split into two separate code paths to avoid conditional select with refract
    if (cannot_refract) {
        direction = reflect(unit_direction, hit_record.normal);
    } else {
        direction = refract(unit_direction, hit_record.normal, refraction_ratio);
    }

    // Choose the correct offset direction based on whether we're entering or exiting
    let offset_direction = select(-hit_record.normal, hit_record.normal, hit_record.front_face);

    // Ensure direction is normalized and valid
    *scattered = Ray(hit_record.p + offset_direction * 0.001, normalize(direction));

    return true;
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

const NUMBER_OF_SPHERES = 4;
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
        randomFloat(seed),
        randomFloat(seed + 1u),
        randomFloat(seed + 2u)
    );
}

// Generate a random unit vector (a point on the unit sphere)
fn random_unit_vector(seed: u32) -> vec3<f32> {
    // Generate 3 random components
    let z = 2.0 * randomFloat(seed) - 1.0;
    let a = randomFloat(seed + 1u) * 6.283185307;
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

const MAX_DEPTH: u32 = 10; // Reduce depth initially if still timing out
fn ray_color(ray: Ray, sphere_list: ptr<function, SphereList>, seed: u32) -> vec3<f32> {
    // ... (no changes needed in logic, but ensure seed usage is robust)
    var current_seed = seed; // Use a mutable seed

    var r = ray;
    var color_factor = vec3<f32>(1.0);

    for (var i: u32 = 0; i < MAX_DEPTH; i++) {
        var hit_record = HitRecord(0.0, vec3<f32>(0.0), vec3<f32>(0.0), false, Material(vec4<f32>(0.0)));
        if (!sphere_list_hit(sphere_list, r, 0.001, 3.4e38, &hit_record)) { // Use 0.001 tmin
             // Hit sky
            let unit_direction = normalize(r.direction);
            let a = 0.5*(unit_direction.y + 1.0);
            let sky_color = (1.0-a)*vec3<f32>(1.0, 1.0, 1.0) + a*vec3<f32>(0.5, 0.7, 1.0);
            return color_factor * sky_color;
        }

        // Use unique seed per bounce
        current_seed = hash(current_seed + i * 100u + u32(hit_record.t * 1000.0)); // Convert to u32 for better precision

        var scattered = Ray(vec3<f32>(0.0), vec3<f32>(0.0));
        var attenuation = vec3<f32>(0.0);
        var scattered_flag = false; // Explicit flag

        // Scatter logic (ensure seed is passed and used)
        if (hit_record.material.albedo.w < -1.0) { // Lambertian (using w now)
            scattered_flag = lambertian_scatter(hit_record.material, r, hit_record, &attenuation, &scattered, current_seed);
        } else if (hit_record.material.albedo.w <= 1.0) { // Metal (w is fuzz/roughness)
            scattered_flag = metal_scatter(hit_record.material, r, hit_record, &attenuation, &scattered, current_seed);
        } else { // Dielectric (w > 1.0 is refractive index)
             // Let's redefine material encoding slightly for clarity
             // material.albedo.x = index of refraction if dielectric
             // material.albedo.w = > 1.0 signifies dielectric type
            scattered_flag = dielectric_scatter(hit_record.material, r, hit_record, &attenuation, &scattered, current_seed);
        }

        if (!scattered_flag) {
            // Absorbed or invalid scatter
             return vec3<f32>(0.0);
        }

        color_factor *= attenuation;
        r = scattered;

        // Russian Roulette or other termination could be added here
        if (max(color_factor.x, max(color_factor.y, color_factor.z)) < 0.01) {
             break; // Stop if contribution is too low
        }
    }

    // If max depth reached without hitting sky
    return vec3<f32>(0.0); // Return black if max depth reached
}


// --- sample_square, get_ray (adjust seed usage) ---
fn sample_square(seed: u32) -> vec2<f32> { // Return vec2
    // Ensure different seeds for x and y using hash properties
    let x = f32(hash(seed)) / 4294967295.0 - 0.5;
    let y = f32(hash(seed + 1u)) / 4294967295.0 - 0.5; // Use different seed for y
    return vec2<f32>(x, y);
}

fn get_ray(location: vec2<i32>, viewport_upper_left: vec3<f32>, pixel_delta_u: vec3<f32>, pixel_delta_v: vec3<f32>, camera_center: vec3<f32>, pixel_seed: u32) -> Ray {
    // Use the provided pixel_seed directly for the offset this frame
    let offset = sample_square(pixel_seed);

    // Calculate pixel sample point for THIS frame
    let pixel_sample_location = viewport_upper_left
        + pixel_delta_u * (f32(location.x) + offset.x + 0.5) // Add offset here
        + pixel_delta_v * (f32(location.y) + offset.y + 0.5);

    let ray_origin = camera_center; // Could add depth of field origin variation here later
    let ray_direction = pixel_sample_location - ray_origin; // Direction towards sample point

    return Ray(ray_origin, normalize(ray_direction)); // Normalize direction
}





// --- update function (CORE CHANGES HERE) ---
@compute @workgroup_size(8, 8, 1) // Match WORKGROUP_SIZE in Rust
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let size = textureDimensions(output);

    // Check bounds (optional but good practice)
    if (location.x >= i32(size.x) || location.y >= i32(size.y)) {
        return;
    }

    // Extract parameters from uniform
    let frame_count = u32(camera.frame_count_reset_samples.x);
    let should_reset = camera.frame_count_reset_samples.y >= 0.5; // Check reset flag (use 0.5 threshold for safety)
    let samples_per_pixel = u32(camera.frame_count_reset_samples.w);

    // --- Camera and Viewport Setup (mostly unchanged) ---
    let aspect_ratio = f32(size.x) / f32(size.y);
    let focal_length = camera.focal_length;
    let viewport_height = camera.viewport_height;
    let viewport_width = viewport_height * aspect_ratio;
    let camera_center = camera.position;

    let w = normalize(-camera.view_direction);
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let u_vec = normalize(cross(up, w)); // Renamed from u to avoid conflict
    let v = cross(w, u_vec);

    let viewport_u = viewport_width * u_vec;
    let viewport_v = -viewport_height * v;

    let pixel_delta_u = viewport_u / f32(size.x);
    let pixel_delta_v = viewport_v / f32(size.y);

    let viewport_upper_left = camera_center
        - focal_length * w       // Along view direction
        - viewport_u / 2.0       // Move left
        - viewport_v / 2.0;      // Move up (since viewport_v is negated)


    // --- Scene Definition (keep as is, or move to storage buffer later) ---
    let material_ground = Material(vec4<f32>(0.8, 0.8, 0.0, -2.0)); // Lambertian
    let material_center = Material(vec4<f32>(0.1, 0.2, 0.5, -2.0)); // Lambertian
    let material_left   = Material(vec4<f32>(1.5, 0.8, 0.8, 1.5));  // Dielectric (IOR 1.5)
    let material_right  = Material(vec4<f32>(0.8, 0.6, 0.2, 0.2));  // Metal (fuzz 0.2)

    let sphere1 = Sphere(vec3<f32>( 0.0,    0.0, -1.0),   0.5, material_center);
    let sphere2 = Sphere(vec3<f32>( 0.0, -100.5, -1.0), 100.0, material_ground);
    let sphere3 = Sphere(vec3<f32>(-1.0,    0.0, -1.0),   0.5, material_left);
    let sphere4 = Sphere(vec3<f32>( 1.0,    0.0, -1.0),   0.5, material_right);

    var sphere_list = SphereList(array<Sphere, NUMBER_OF_SPHERES>(sphere1, sphere2, sphere3, sphere4));


    // --- Accumulation Logic ---
    var current_data = textureLoad(input, location);
    var accum_color = current_data.rgb;
    var sample_count = u32(current_data.a); // Sample count stored in alpha

    // Reset if requested OR if frame_count is 0 (handles initial state too)
    if (should_reset || frame_count == 0u) {
        accum_color = vec3<f32>(0.0);
        sample_count = 0u;
    }

    // Calculate one sample if not already finished
    if (sample_count < samples_per_pixel) {
        // Generate a unique seed for this pixel and this frame/sample_count
        // Combine location, frame_count for temporal variation
        let pixel_base_seed = hash(u32(location.x) * 1973u + u32(location.y) * 9277u);
        let frame_seed = hash(pixel_base_seed + frame_count * 1103u); // Use frame_count for unique sample each frame

        // Get ray for this specific sample
        let r = get_ray(location, viewport_upper_left, pixel_delta_u, pixel_delta_v, camera_center, frame_seed);

        // Calculate color for this sample
        let new_sample_color = ray_color(r, &sphere_list, frame_seed); // Pass seed to ray_color

        // Add to accumulator (avoiding NaNs)
        // if (!any(isnan(new_sample_color))) {
             accum_color += new_sample_color;
        // }
        sample_count += 1u;
    }

    // Store updated accumulation data (color sum and count)
    textureStore(output, location, vec4<f32>(accum_color, f32(sample_count)));
}
