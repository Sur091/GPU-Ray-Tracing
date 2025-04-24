use bevy::{
    prelude::*,
    render::{extract_resource::ExtractResource, render_resource::ShaderType},
};
use bytemuck::{Pod, Zeroable};

// Camera settings used in the main app
#[derive(Resource, Debug, Clone)]
pub struct CameraSettings {
    pub camera_center: Vec3,
    pub focal_length: f32,
    pub view_direction: Vec3,
    pub viewport_height: f32,
    pub number_of_samples: u32,
    pub camera_has_moved: bool,
    pub max_depth: u32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            camera_center: Vec3::ZERO,
            focal_length: 1.0,
            view_direction: -Vec3::Z,
            viewport_height: 2.0,
            number_of_samples: 100,
            camera_has_moved: false,
            max_depth: 50,
        }
    }
}

// GPU-compatible camera representation that matches shader's expectations
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Resource, ExtractResource, ShaderType, Pod, Zeroable)]
pub struct SceneCamera {
    pub position: Vec3,
    pub focal_length: f32,
    pub view_direction: Vec3,
    pub viewport_height: f32,
    pub reset_seed_depth_samples: Vec4, // x: reset flag, y: random seed, z: max_depth, w: samples
}

impl From<&CameraSettings> for SceneCamera {
    fn from(settings: &CameraSettings) -> Self {
        Self {
            position: settings.camera_center,
            focal_length: settings.focal_length,
            view_direction: settings.view_direction,
            viewport_height: settings.viewport_height,
            reset_seed_depth_samples: Vec4::new(
                if settings.camera_has_moved { 1.0 } else { 0.0 }, // The reset flag
                rand::random::<f32>(), // The random seed
                settings.max_depth as f32, // max_depth
                settings.number_of_samples as f32, // The number of samples
            ),
        }
    }
}
