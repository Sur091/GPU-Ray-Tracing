// The shader reads the previous frame's state from the `input` texture, and writes the new state of
// each pixel to the `output` texture. The textures are flipped each step to progress the
// simulation.
// Two textures are needed for the game of life as each pixel of step N depends on the state of its
// neighbors at step N-1.

@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;

@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

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

fn hit_sphere(center: vec3<f32>, radius: f32, r: Ray) -> f32 {
    let oc = center - r.origin;
    let a = dot(r.direction, r.direction);
    let b = -2.0 * dot(oc, r.direction);
    let c = dot(oc, oc) - radius * radius;
    let discriminant = b*b - 4.0*a*c;
    
    if discriminant >= 0.0 {
        return (-b - sqrt(discriminant)) / (2.0 * a);
    }
    return -1.0;
}


fn ray_color(r: Ray) -> vec3<f32> {
    let t = hit_sphere(vec3<f32>(0.0, 0.0, -1.0), 0.5, r);
    if t >= 0.0 {
        let normal = normalize(r.origin + t * r.direction - vec3<f32>(0.0, 0.0, -1.0));
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
    
    // Camera setup
    let focal_length = 1.0;
    let viewport_height = 2.0;
    let viewport_width = viewport_height * aspect_ratio;
    let camera_center = vec3<f32>(0.0, 0.0, 0.0);
    
    // Calculate viewport vectors
    let viewport_u = vec3<f32>(viewport_width, 0.0, 0.0);
    let viewport_v = vec3<f32>(0.0, -viewport_height, 0.0); // Negative to flip y-axis
    
    // Calculate pixel deltas
    let pixel_delta_u = viewport_u / f32(size.x);
    let pixel_delta_v = viewport_v / f32(size.y);
    
    // Calculate viewport upper left corner
    let viewport_upper_left = camera_center 
        - viewport_u / 2.0 
        - viewport_v / 2.0 
        - vec3<f32>(0.0, 0.0, focal_length);
    
    // Calculate pixel center
    let pixel_center = viewport_upper_left 
        + pixel_delta_u * (f32(location.x) + 0.5) 
        + pixel_delta_v * (f32(location.y) + 0.5);
    
    let ray_direction = pixel_center - camera_center;
    let r = Ray(camera_center, ray_direction);
    
    let color = vec4<f32>(ray_color(r), 1.0);
    textureStore(output, location, color);
}