// The shader reads the previous frame's state from the `input` texture, and writes the new state of
// each pixel to the `output` texture. The textures are flipped each step to progress the
// simulation.
// Two textures are needed for the game of life as each pixel of step N depends on the state of its
// neighbors at step N-1.

@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;

@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;


struct CameraUniform {
    position: vec3<f32>,
    focal_length: f32,
    view_direction: vec3<f32>,
    viewport_height: f32,
    _padding: vec4<f32>,
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

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let color = vec4<f32>(f32(false));

    textureStore(output, location, color);
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>
}

struct HitRecord {
    t: f32,
    p: vec3<f32>,
    normal: vec3<f32>,
}

struct Sphere {
    center: vec3<f32>,
    radius: f32
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
    // *rec.t = root;
    // *rec.p = r.origin + root * r.direction;
    // *rec.normal = ((*rec.p) - sphere.center) / sphere.radius;
    *rec = HitRecord(root, r.origin + root * r.direction, ((*rec).p - sphere.center) / sphere.radius);
    
    return true;
}



fn ray_color(r: Ray) -> vec3<f32> {
    var hit_record = HitRecord(0.0, vec3<f32>(0.0), vec3<f32>(0.0));
    let sphere = Sphere(vec3<f32>(0.0, 0.0, -1.0), 0.5);
    let t = sphere_hit(sphere, r, 0.0, 100.0, &hit_record);
    if t {
        let normal = normalize(r.origin + 0.5 * r.direction - vec3<f32>(0.0, 0.0, -1.0));
        return 0.5 * (normal + 1.0);
    }

    let unit_direction = normalize(r.direction);
    let a = 0.5*(unit_direction.y + 1.0);
    return (1.0-a)*vec3<f32>(1.0, 1.0, 1.0) + a*vec3<f32>(0.5, 0.7, 1.0);
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
    let r = Ray(camera_center, ray_direction);
    
    let sphere = Sphere(vec3<f32>(0.0, 0.0, -1.0), 0.5);
    
    let color = vec4<f32>(ray_color(r), 1.0);
    textureStore(output, location, color*color);
}