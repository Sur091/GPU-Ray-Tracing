use bevy::{
    input::mouse::{MouseMotion, MouseWheel}, prelude::*, render::{extract_resource::ExtractResource, render_resource::ShaderType}
};
use bytemuck::{Pod, Zeroable};

// Camera settings used in the main app
#[derive(Resource, Debug, Clone)]
pub struct CameraSettings {
    pub camera_center: Vec3,
    pub focal_length: f32,
    pub view_direction: Vec3,
    pub field_of_view: f32,
    pub number_of_samples: u32,
    pub camera_has_moved: bool,
    pub max_depth: u32,
    // Camera movement is handled by keyboard and mouse controls:
    // W/S: Move forward/backward
    // A/D: Strafe left/right
    // Up/Down arrows: Move up/down
    // Left/Right arrows: Rotate camera
    // Mouse wheel: Zoom in/out (change field of view)
    // Right mouse button + drag: Rotate camera view
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            camera_center: Vec3::new(0.0, 1.0, 3.0),
            focal_length: 1.0,
            view_direction: Vec3::new(0.0, 0.0, -1.0).normalize(),
            field_of_view: 60.0,
            number_of_samples: 200,
            camera_has_moved: true, // Start with reset flag on to render first frame
            max_depth: 50,
        }
    }
}
/// System to handle camera control with mouse (wheel zoom, movement)
pub fn camera_mouse_controls_system(
    mut mouse_wheel: EventReader<MouseWheel>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut camera_settings: ResMut<CameraSettings>,
) {
    let mut moved = false;

    // Handle mouse wheel for zooming (changing field of view)
    for event in mouse_wheel.read() {
        // Adjust field of view based on scroll direction
        // Scrolling up (positive y) decreases FOV (zooms in)
        // Scrolling down (negative y) increases FOV (zooms out)
        let zoom_delta = -event.y * MOUSE_ZOOM_SENSITIVITY;
        let new_fov = (camera_settings.field_of_view + zoom_delta).clamp(FOV_MIN, FOV_MAX);

        if new_fov != camera_settings.field_of_view {
            camera_settings.field_of_view = new_fov;
            moved = true;
        }
    }

    // Handle mouse movement while right button is pressed
    if mouse_button.pressed(MouseButton::Right) {
        // Calculate camera rotation from mouse movement
        for event in mouse_motion.read() {
            // Horizontal movement (x) rotates around Y axis (yaw)
            if event.delta.x != 0.0 {
                let rotation = Quat::from_rotation_y(-event.delta.x * MOUSE_MOVE_SENSITIVITY);
                camera_settings.view_direction = rotation
                    .mul_vec3(camera_settings.view_direction)
                    .normalize();
                moved = true;
            }

            // Vertical movement (y) rotates around local X axis (pitch)
            if event.delta.y != 0.0 {
                // Get current camera basis vectors
                let forward = camera_settings.view_direction.normalize();
                let right = forward.cross(Vec3::Y).normalize();

                // Create rotation around the right vector (pitch)
                let rotation =
                    Quat::from_axis_angle(right, -event.delta.y * MOUSE_MOVE_SENSITIVITY);

                // Apply rotation - but check to prevent flipping over
                let new_direction = rotation.mul_vec3(forward).normalize();

                // Prevent camera from flipping by checking if the new direction is not too close to up/down
                if new_direction.dot(Vec3::Y).abs() < 0.95 {
                    camera_settings.view_direction = new_direction;
                    moved = true;
                }
            }
        }
    }

    // Update the camera_has_moved flag if needed
    if moved {
        camera_settings.camera_has_moved = true;
    }
}

// Camera movement constants
const CAMERA_MOVE_SPEED: f32 = 2.0; // Units per second
const CAMERA_ROTATE_SPEED: f32 = 1.0; // Radians per second
const CAMERA_VERTICAL_SPEED: f32 = 1.0; // Units per second
const MOUSE_ZOOM_SENSITIVITY: f32 = 1.0; // FOV change per scroll unit
const MOUSE_MOVE_SENSITIVITY: f32 = 0.002; // Movement sensitivity
const FOV_MIN: f32 = 10.0; // Minimum field of view (degrees)
const FOV_MAX: f32 = 120.0; // Maximum field of view (degrees)

/// System to handle camera movement based on keyboard input
pub fn camera_movement_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera_settings: ResMut<CameraSettings>,
) {
    let dt = time.delta_secs();
    let mut moved = false;

    // Get current camera basis vectors
    let forward = camera_settings.view_direction.normalize();
    let right = forward.cross(Vec3::Y).normalize();
    // let up = right.cross(forward).normalize();

    // Handle forward/backward movement (W/S)
    if keyboard_input.pressed(KeyCode::KeyW) {
        camera_settings.camera_center += forward * CAMERA_MOVE_SPEED * dt;
        moved = true;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        camera_settings.camera_center -= forward * CAMERA_MOVE_SPEED * dt;
        moved = true;
    }

    // Handle strafing left/right (A/D)
    if keyboard_input.pressed(KeyCode::KeyA) {
        camera_settings.camera_center -= right * CAMERA_MOVE_SPEED * dt;
        moved = true;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        camera_settings.camera_center += right * CAMERA_MOVE_SPEED * dt;
        moved = true;
    }

    // Handle vertical movement (Up/Down arrows)
    if keyboard_input.pressed(KeyCode::ArrowUp) {
        camera_settings.camera_center += Vec3::Y * CAMERA_VERTICAL_SPEED * dt;
        moved = true;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        camera_settings.camera_center -= Vec3::Y * CAMERA_VERTICAL_SPEED * dt;
        moved = true;
    }

    // Handle rotation (Left/Right arrows)
    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        // Rotate around Y axis (yaw)
        let rotation = Quat::from_rotation_y(CAMERA_ROTATE_SPEED * dt);
        camera_settings.view_direction = rotation.mul_vec3(camera_settings.view_direction);
        moved = true;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        // Rotate around Y axis (yaw) - opposite direction
        let rotation = Quat::from_rotation_y(-CAMERA_ROTATE_SPEED * dt);
        camera_settings.view_direction = rotation.mul_vec3(camera_settings.view_direction);
        moved = true;
    }

    // Update the reset flag if movement occurred
    if moved {
        camera_settings.camera_has_moved = true;
    } else {
        // Reset the flag if no movement this frame and it was previously set
        if camera_settings.camera_has_moved {
            camera_settings.camera_has_moved = false;
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
    pub field_of_view: f32,
    pub reset_seed_depth_samples: Vec4, // x: reset flag, y: random seed, z: max_depth, w: samples
}

impl From<&CameraSettings> for SceneCamera {
    fn from(settings: &CameraSettings) -> Self {
        Self {
            position: settings.camera_center,
            focal_length: settings.focal_length,
            view_direction: settings.view_direction,
            field_of_view: settings.field_of_view,
            reset_seed_depth_samples: Vec4::new(
                if settings.camera_has_moved { 1.0 } else { 0.0 }, // The reset flag
                rand::random::<f32>(),                             // The random seed
                settings.max_depth as f32,                         // max_depth
                settings.number_of_samples as f32,                 // The number of samples
            ),
        }
    }
}


// Extract camera settings into the render world
pub fn extract_camera(camera_settings: Res<CameraSettings>, mut commands: Commands) {
    // Convert CameraSettings to the GPU-compatible SceneCamera
    let scene_camera =SceneCamera::from(camera_settings.as_ref());

    // Insert as a resource that will be extracted to the render world
    commands.insert_resource(scene_camera);
}

