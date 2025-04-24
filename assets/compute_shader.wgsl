// Compute shader for GPU ray tracing

// Texture bindings for input/output
@group(0) @binding(0) var input: texture_storage_2d<rgba32float, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba32float, write>;

// Camera uniform buffer
struct SceneCamera {
    position: vec3<f32>,
    focal_length: f32,
    view_direction: vec3<f32>,
    viewport_height: f32,
    reset_un_un_samples: vec4<f32>, // x: reset flag, y: unused, z: unused, w: samples
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

struct HitRecord {
    t: f32,
    p: vec3<f32>,
    normal: vec3<f32>,
    front_face: bool
}

struct Sphere {
    center: vec3<f32>,
    radius: f32
}

const NUMBER_OF_SPHERES = 2;
struct SphereList {
    spheres: array<Sphere, NUMBER_OF_SPHERES>
}

fn hit_record_set_face_normal(rec: ptr<function, HitRecord>, r: Ray, outward_normal: vec3<f32>) {
    let front_face = dot(r.direction, outward_normal) < 0.0;
    let normal = select(-outward_normal, outward_normal, front_face);
    *rec = HitRecord((*rec).t, (*rec).p, normal, front_face);
}

fn sphere_list_hit(sphere_list: ptr<function, SphereList>, r: Ray, ray_tmin: f32, ray_tmax: f32, rec: ptr<function, HitRecord>) -> bool {
    var temp_rec = HitRecord(0.0, vec3<f32>(0.0), vec3<f32>(0.0), false);
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
        false
    );

    // Set the face normal
    hit_record_set_face_normal(rec, r, outward_normal);

    return true;
}



fn ray_color(r: Ray, sphere_list: ptr<function, SphereList>) -> vec3<f32> {
    var hit_record = HitRecord(0.0, vec3<f32>(0.0), vec3<f32>(0.0), false);
    let t = sphere_list_hit(sphere_list, r, 0.001, 3.4e35, &hit_record);
    if t {
        return 0.5 * (hit_record.normal + 1.0);
    }

    let unit_direction = normalize(r.direction);
    let a = 0.5*(unit_direction.y + 1.0);
    return (1.0-a)*vec3<f32>(1.0, 1.0, 1.0) + a*vec3<f32>(0.5, 0.7, 1.0);
}

fn sample_square(seed: u32) -> vec3<f32> {
    let x = random_float(seed) - 0.5;
    let y = random_float(seed * seed) - 0.5;
    return vec3<f32>(x, y, 0.0);
}

fn get_ray(location: vec2<i32>, viewport_upper_left: vec3<f32>, pixel_delta_u: vec3<f32>, pixel_delta_v: vec3<f32>, camera_center: vec3<f32>, seed: u32) -> Ray {
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
    let viewport_height = camera.viewport_height;
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

    // Calculate pixel center
    let pixel_center = viewport_upper_left
        + pixel_delta_u * (f32(location.x) + 0.5)
        + pixel_delta_v * (f32(location.y) + 0.5);

    let ray_direction = pixel_center - camera_center;
    
    // Check the progress
    let progress = textureLoad(input, location);
    var color_until_now = progress.xyz;
    var samples_until_now = u32(progress.w);
    

    let sphere1 = Sphere(vec3<f32>(0.0, 0.0, -1.0), 0.5);
    let sphere2 = Sphere(vec3<f32>(0.0, -100.5, -1.0), 100.0);

    var sphere_list = SphereList(array<Sphere, NUMBER_OF_SPHERES>(sphere1, sphere2));
        
    let samples_per_pixel = u32(camera.reset_un_un_samples.w);
    
    if (samples_until_now < samples_per_pixel) {
        let seed = 1u + samples_until_now;
        let ray = get_ray(location, viewport_upper_left, pixel_delta_u, pixel_delta_v, camera_center, seed);
        let color = ray_color(ray, &sphere_list);
        color_until_now += (color - color_until_now) / f32(samples_until_now + 1u);
        samples_until_now += 1u;
    }
    
    let final_color = vec4<f32>(color_until_now, f32(samples_until_now));
    textureStore(output, location, final_color);
}