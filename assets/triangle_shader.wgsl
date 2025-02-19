// Vertex Shader
// @vertex
// fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
//     // Define the vertices of the triangle
//     let vertices = array<vec2<f32>, 3>(
//         vec2<f32>(-0.5, -0.5),
//         vec2<f32>( 0.5, -0.5),
//         vec2<f32>( 0.0,  0.5)
//     );

//     let pos = vertices[vertex_index];
//     return vec4<f32>(pos, 0.0, 1.0);
// }

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var<uniform> material_color: vec4<f32>;

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    return material_color;
}