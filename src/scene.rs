use std::f32::consts::PI;

use bevy::color::palettes::css::*;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;

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

pub fn extract_camera(mut commands: Commands, camera_query: Query<&Transform, With<Camera3d>>) {
    if let Ok(camera_transform) = camera_query.get_single() {
        commands.insert_resource(camera::SceneCamera {
            position: camera_transform.translation,
            view_direction: camera_transform.forward().into(),
            focal_length: 1.0,
            viewport_height: 2.0,
            _padding: Vec3::ZERO,
            samples_per_pixel: 50,
        });
    }
}
