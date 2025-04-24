#import bevy_asset::asset::MaterialAssets
#import bevy_render::view::View
// #import bevy_pbr::mesh_view::globals
#import bevy_sprite::sprite_mesh2d::Mesh2dViewBindGroup

// NOTE: This shader is basically a copy of the relevant parts of
// bevy_sprite::assets::shaders::mesh2d.wgsl as of Bevy 0.15
// to provide a starting point for customization.

// Bind groups (match the expected layout for 2D meshes)
@group(0) @binding(0) var<uniform> view: View; // From Camera2dBundle
// @group(1) @binding(0) var<uniform> globals: Globals; // Global time etc. (optional here)

// Vertex Input attributes (match Mesh data)
struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>, // Included for standard mesh layout, unused here
    @location(2) uv: vec2<f32>,
};

// Vertex Output (to fragment shader)
struct VertexOutput {
    // The vertex shader must set the on-screen position of the vertex.
    @builtin(position) clip_position: vec4<f32>,
    // Pass UV coordinates to the fragment shader.
    @location(0) uv: vec2<f32>,
};

// Vertex Shader function
@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    // Calculate the final position of the vertex in clip space.
    // For 2D, we usually use view.view_proj * model_matrix * vertex_position.
    // Since we are drawing a screen-space quad centered at origin with Transform::ZERO,
    // the model matrix is identity. view_proj transforms from world to clip space.
    // We use vec4 for matrix multiplication.
    out.clip_position = mat4x4<f32>(view.viewport.z, 0.0, 0.0, 0.0,
                                         0.0, view.viewport.w, 0.0, 0.0,
                                         0.0, 0.0, 1.0, 0.0,
                                         0.0, 0.0, 0.0, 1.0) * vec4<f32>(vertex.position, 1.0);

    // Pass the UV coordinates directly to the fragment shader.
    out.uv = vertex.uv;
    return out;
}

// Dummy fragment entry point needed by Material2d pipeline unless `fragment_shader` is overridden.
// Since we *are* overriding fragment_shader in Rust, this isn't strictly necessary
// for *this specific material*, but it's good practice for a standalone vertex shader file.
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(1.0, 0.0, 1.0, 1.0); // Magenta fallback
}
