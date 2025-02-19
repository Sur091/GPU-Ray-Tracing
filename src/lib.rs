// A shader in Bevy

use bevy::prelude::*;
use bevy::sprite::Material2dPlugin;

mod custom_material;
use custom_material::CustomMaterial;

pub fn run() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            Material2dPlugin::<CustomMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::default())),
        MeshMaterial2d(materials.add(CustomMaterial {
            color: LinearRgba::BLUE,
        })),
        Transform::default().with_scale(Vec3::splat(256.)),
    ));

    commands.spawn(Camera2d);
}
