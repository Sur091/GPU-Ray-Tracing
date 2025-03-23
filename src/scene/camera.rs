use bevy::{
    prelude::*,
    render::{extract_resource::ExtractResource, render_resource::ShaderType},
};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Resource, ExtractResource, ShaderType, Pod, Zeroable)]
pub struct SceneCamera {
    pub position: Vec3,
    pub focal_length: f32,
    pub view_direction: Vec3,
    pub viewport_height: f32,
    pub _padding: Vec4,
}

impl Default for SceneCamera {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            focal_length: 1.0,
            view_direction: -Vec3::Z,
            viewport_height: 2.0,
            _padding: Vec4::ZERO,
        }
    }
}
