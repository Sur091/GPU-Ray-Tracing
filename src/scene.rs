use std::f32::consts::PI;

use bevy::color::palettes::css::*;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;

// use rand::Rng;

pub mod camera;
pub mod compute_shader;

// #[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash)]
// struct Scene;

// Set up the scene
pub fn _setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a sphere by default
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5).mesh().uv(32, 18))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: LIMEGREEN.into(),
            ..default()
        })),
        Transform::from_xyz(0.0, 1.0, 1.5),
    ));

    // Spawn a big sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(10.5).mesh().uv(64, 36))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: SKY_BLUE.into(),
            ..default()
        })),
        Transform::from_xyz(0.0, -12.0, -13.5),
    ));

    // ambient light
    commands.insert_resource(AmbientLight {
        color: SKY_BLUE.into(),
        brightness: 100.0,
    });

    // directional 'sun' light
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        // The default cascade config is designed to handle large scenes.
        // As this example has a much smaller world, we can tighten the shadow
        // bounds for better visual quality.
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 4.0,
            maximum_distance: 10.0,
            ..default()
        }
        .build(),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

pub fn setup_camera_settings(mut commands: Commands) {
    commands.insert_resource(camera::CameraSettings::default());
}

// System to update camera settings and detect changes for reset
pub fn update_camera_settings(
    mut settings: ResMut<camera::CameraSettings>,
    camera_query: Query<&Transform, With<Camera3d>>,
    // Add inputs to move camera later if needed
    // input: Res<ButtonInput<KeyCode>>,
) {
    if let Ok(transform) = camera_query.get_single() {
        let mut changed = false;
        if transform.translation != settings.position {
            settings.position = transform.translation;
            changed = true;
        }
        let forward = transform.forward().into();
        if forward != settings.view_direction {
            settings.view_direction = forward;
            changed = true;
        }

        // Add checks for other changes (fov, movement keys, etc.)
        // if input.just_pressed(KeyCode::W) { changed = true; } // Example

        if changed {
            settings.needs_reset = true;
            settings.frame_count = 0;
            info!("Camera changed, resetting accumulation.");
        } else {
            // Only increment frame count if not resetting
            settings.needs_reset = false; // Reset handled, clear flag for next frame
            settings.frame_count += 1;
        }
    }
}

// Extract the derived SceneCamera uniform data
pub fn extract_camera_uniform(mut commands: Commands, settings: Res<camera::CameraSettings>) {
    // Ensure the resource is updated or inserted for the render world
    commands.insert_resource(camera::SceneCamera::from(settings.as_ref()));
}
