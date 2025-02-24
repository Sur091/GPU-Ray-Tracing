// A shader in Bevy

use bevy::prelude::*;
use bevy::sprite::Material2dPlugin;
use bevy::window::{PrimaryWindow, WindowResized};

mod custom_material;
use custom_material::CustomMaterial;

pub fn run() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            Material2dPlugin::<CustomMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, resize_rectangle)
        .run();
}

#[derive(Component)]
struct ResizableRectangle;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    // Ensure that there is exactly one primary window
    let window = windows.single();
    let (width, height) = (window.width(), window.height());
    let rectangle = Rectangle::new(width, height);
    commands.spawn((
        Mesh2d(meshes.add(rectangle)),
        MeshMaterial2d(materials.add(CustomMaterial {
            color: LinearRgba::BLUE,
        })),
        Transform::default(),
        ResizableRectangle,
    ));

    commands.spawn(Camera2d);
}

fn resize_rectangle(
    mut meshes: ResMut<Assets<Mesh>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<&mut Mesh2d, With<ResizableRectangle>>,
    resize_events: EventReader<WindowResized>,
) {
    // Check if there is a resize event
    if resize_events.is_empty() {
        return;
    }

    // Get the new window dimensions
    let window = windows.single();
    let (width, height) = (window.width(), window.height());

    for mut resizable_rectangle in query.iter_mut() {
        let rectangle = Rectangle::new(width, height);
        *resizable_rectangle = Mesh2d(meshes.add(rectangle));
    }
}
