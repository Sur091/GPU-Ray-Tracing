use bevy::prelude::*;

mod scene;

const WINDOW_SIZE: (u32, u32) = (1280, 720); // Adjust as needed

pub fn run() {
    let default_plugin_with_window = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution: (WINDOW_SIZE.0 as f32, WINDOW_SIZE.1 as f32).into(),
            title: "Bevy GPU Ray Tracer".into(), // Add title
            ..default()
        }),
        ..default()
    })
    // No need for ImagePlugin::default_nearest() globally if post-process handles sampling
    ;

    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            default_plugin_with_window,
            scene::compute_shader::ComputeShaderPlugin, // Add our plugin
        ))
        // Setup systems
        .add_systems(Startup, scene::compute_shader::setup_compute_shader)
        .add_systems(Startup, scene::setup_camera_settings) // Add camera settings setup
        // Update systems
        .add_systems(Update, scene::compute_shader::switch_textures)
        .add_systems(Update, scene::update_camera_settings) // Update settings and check reset
        // Extraction system (runs in PostUpdate schedule by default)
        .add_systems(PostUpdate, scene::extract_camera_uniform)
        // Add a system to quit on Esc
        .run();
}
