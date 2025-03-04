// A shader in Bevy

use bevy::prelude::*;

mod scene;

pub fn run() {
    App::new()
        .add_plugins((DefaultPlugins,))
        .add_systems(Startup, scene::setup_scene)
        .run();
}
