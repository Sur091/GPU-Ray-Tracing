// A shader in Bevy

use bevy::prelude::*;

mod custom_material;
use custom_material::CustomMaterial;


pub fn run() {
    App::new()
        .add_plugins((DefaultPlugins, MaterialPlugin::<CustomMaterial>::default()))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {

    commands.spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(materials.add(CustomMaterial {
                color: LinearRgba::RED,
            })),
            Transform::from_xyz(0.0, 0.5, 0.0),
        ));
    
    commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
    
}


