use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

const SHADER_ASSET_PATH: &str = "triangle_shader.wgsl";

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct CustomMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
