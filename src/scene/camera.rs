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

    // Use last component for samples_per_pixel to ensure alignment
    // Pack frame_count and reset flag into the padding space
    pub frame_count_reset_samples: Vec4, // x: frame_count, y: reset_flag (1.0 = reset), z: unused, w: samples_per_pixel_float
}

// Helper struct for easier CPU-side manipulation
#[derive(Resource, Debug, Clone)]
pub struct CameraSettings {
    pub position: Vec3,
    pub view_direction: Vec3,
    pub focal_length: f32,
    pub viewport_height: f32,
    pub samples_per_pixel: u32,
    pub frame_count: u32,
    pub needs_reset: bool,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            focal_length: 1.0,
            view_direction: -Vec3::Z,
            viewport_height: 2.0,
            samples_per_pixel: 50, // Start with fewer samples during dev
            frame_count: 0,
            needs_reset: true, // Start with a reset
        }
    }
}

// Implement conversion for extraction
impl From<&CameraSettings> for SceneCamera {
    fn from(settings: &CameraSettings) -> Self {
        Self {
            position: settings.position,
            focal_length: settings.focal_length,
            view_direction: settings.view_direction,
            viewport_height: settings.viewport_height,
            frame_count_reset_samples: Vec4::new(
                settings.frame_count as f32,
                if settings.needs_reset { 1.0 } else { 0.0 },
                0.0, // unused padding z
                settings.samples_per_pixel as f32,
            ),
        }
    }
}
