// gpu_ray_tracing/assets/post_process.wgsl
#import bevy_sprite::mesh2d_functions // For mesh processing if needed, but basic is fine

// Use group 1 for material textures to avoid conflict with Bevy's group 0 uniforms
@group(2) @binding(0) var source_texture: texture_2d<f32>;
@group(2) @binding(1) var source_sampler: sampler;

// Use the standard vertex shader output structure
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};


// Basic fragment shader
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let data = textureSample(source_texture, source_sampler, in.uv);
    let accum_color = data.rgb;
    let sample_count = data.a;

    var final_color = vec3<f32>(0.0);
    if (sample_count > 0.0) {
        final_color = accum_color / sample_count;
    }

    // Apply gamma correction (simple version)
    let gamma_corrected = pow(final_color, vec3<f32>(1.0 / 2.2));

    // Return final color for display
    return vec4<f32>(gamma_corrected, 1.0);
}
