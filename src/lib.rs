// A shader in Bevy

use bevy::prelude::*;

mod compute_shader;
mod scene;

const WINDOW_SIZE: (u32, u32) = (1280, 720);

pub fn run() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: (WINDOW_SIZE.0 as f32, WINDOW_SIZE.1 as f32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            compute_shader::ComputeShaderPlugin,
        ))
        .add_systems(Startup, compute_shader::setup_compute_shader)
        .add_systems(Update, compute_shader::switch_textures)
        .run();
}
