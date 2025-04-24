use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    render::{extract_resource::ExtractResource, render_resource::ShaderType},
};
use bytemuck::{Pod, Zeroable};

// Camera settings used in the main app
#[derive(Resource, Debug, Clone)]
pub struct CameraSettings {
    pub field_of_view: f32,
    pub samples_per_pixel: u32,
    pub camera_has_moved: bool,
    pub max_depth: u32,
    pub vup: Vec3,
    pub look_from: Vec3,
    pub look_at: Vec3,
    pub defocus_angle: f32,
    pub focus_distance: f32,
    // Camera movement is handled by keyboard and mouse controls:
    // W/S: Move forward/backward
    // A/D: Strafe left/right
    // Up/Down arrows: Move up/down
    // Left/Right arrows: Rotate camera left/right (yaw)
    // PageUp/PageDown: Look up/down (pitch)
    // Mouse wheel: Zoom in/out (change field of view)
    // Right mouse button + drag: Rotate camera view
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            samples_per_pixel: 200,
            camera_has_moved: true, // Start with reset flag on to render first frame
            max_depth: 50,
            vup: Vec3::new(0.0, 1.0, 0.0),
            field_of_view: 20.0,
            look_from: Vec3::new(-2.0, 2.0, 1.0),
            look_at: Vec3::new(0.0, 0.0, -1.0),
            defocus_angle: 10.0,
            focus_distance: 3.4,
        }
    }
}
/// System to handle camera control with mouse (wheel zoom, movement)
pub fn _camera_mouse_controls_system(
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
                let view_direction = camera_settings.look_at - camera_settings.look_from;
                let len = view_direction.length();
                let view_direction = rotation.mul_vec3(view_direction).normalize();
                camera_settings.look_at = camera_settings.look_from + view_direction * len;
                moved = true;
            }

            // Vertical movement (y) rotates around local X axis (pitch)
            if event.delta.y != 0.0 {
                // Get current camera basis vectors
                let view_direction = camera_settings.look_at - camera_settings.look_from;
                let len = view_direction.length();
                let forward = view_direction.normalize();
                let right = forward.cross(Vec3::Y).normalize();

                // Create rotation around the right vector (pitch)
                let rotation =
                    Quat::from_axis_angle(right, -event.delta.y * MOUSE_MOVE_SENSITIVITY);

                // Apply rotation - but check to prevent flipping over
                let new_direction = rotation.mul_vec3(forward).normalize();

                // Prevent camera from flipping by checking if the new direction is not too close to up/down
                if new_direction.dot(Vec3::Y).abs() < 0.95 {
                    let view_direction = rotation.mul_vec3(view_direction).normalize();
                    camera_settings.look_at = camera_settings.look_from + view_direction * len;
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
    let view_direction = camera_settings.look_from - camera_settings.look_at;
    let forward = view_direction.normalize();
    let right = forward.cross(Vec3::Y).normalize();
    // let up = right.cross(forward).normalize();

    // Handle forward/backward movement (W/S)
    if keyboard_input.pressed(KeyCode::KeyW) {
        camera_settings.look_from += forward * CAMERA_MOVE_SPEED * dt;
        moved = true;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        camera_settings.look_from -= forward * CAMERA_MOVE_SPEED * dt;
        moved = true;
    }

    // Handle strafing left/right (A/D)
    if keyboard_input.pressed(KeyCode::KeyA) {
        camera_settings.look_from -= right * CAMERA_MOVE_SPEED * dt;
        moved = true;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        camera_settings.look_from += right * CAMERA_MOVE_SPEED * dt;
        moved = true;
    }

    // Handle vertical movement (Up/Down arrows)
    if keyboard_input.pressed(KeyCode::ArrowUp) {
        camera_settings.look_from += Vec3::Y * CAMERA_VERTICAL_SPEED * dt;
        moved = true;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        camera_settings.look_from -= Vec3::Y * CAMERA_VERTICAL_SPEED * dt;
        moved = true;
    }

    // Handle rotation (Left/Right arrows)
    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        // Rotate around Y axis (yaw)
        let rotation = Quat::from_rotation_y(CAMERA_ROTATE_SPEED * dt);
        let view_direction = camera_settings.look_from - camera_settings.look_at;
        let len = view_direction.length();
        let view_direction = rotation.mul_vec3(view_direction).normalize();
        camera_settings.look_from = camera_settings.look_at + view_direction * len;
        moved = true;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        // Rotate around Y axis (yaw) - opposite direction
        let rotation = Quat::from_rotation_y(-CAMERA_ROTATE_SPEED * dt);
        let view_direction = camera_settings.look_from - camera_settings.look_at;
        let len = view_direction.length();
        let view_direction = rotation.mul_vec3(view_direction).normalize();
        camera_settings.look_from = camera_settings.look_at + view_direction * len;
        moved = true;
    }
    // Handle rotation (Left/Right arrows)
    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        // Rotate around Y axis (yaw)
        let rotation = Quat::from_rotation_y(CAMERA_ROTATE_SPEED * dt);
        let view_direction = camera_settings.look_from - camera_settings.look_at;
        let len = view_direction.length();
        let view_direction = rotation.mul_vec3(view_direction).normalize();
        camera_settings.look_from = camera_settings.look_at + view_direction * len;
        moved = true;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        // Rotate around Y axis (yaw) - opposite direction
        let rotation = Quat::from_rotation_y(-CAMERA_ROTATE_SPEED * dt);
        let view_direction = camera_settings.look_from - camera_settings.look_at;
        let len = view_direction.length();
        let view_direction = rotation.mul_vec3(view_direction).normalize();
        camera_settings.look_from = camera_settings.look_at + view_direction * len;
        moved = true;
    }

    // Handle looking up/down (PageUp/PageDown)
    if keyboard_input.pressed(KeyCode::Digit1) {
        // Get right vector (perpendicular to view direction and world up)
        let view_direction = camera_settings.look_from - camera_settings.look_at;
        let len = view_direction.length();
        let forward = view_direction.normalize();
        let right = forward.cross(Vec3::Y).normalize();

        // Create rotation around the right vector (pitch up)
        let rotation = Quat::from_axis_angle(right, CAMERA_ROTATE_SPEED * dt);
        let new_direction = rotation.mul_vec3(forward).normalize();

        // Prevent camera from flipping by checking if the new direction is not too close to up/down
        if new_direction.dot(Vec3::Y).abs() < 0.95 {
            camera_settings.look_from = camera_settings.look_at + new_direction * len;
            moved = true;
        }
    }
    if keyboard_input.pressed(KeyCode::Digit2) {
        // Get right vector (perpendicular to view direction and world up)
        let view_direction = camera_settings.look_from - camera_settings.look_at;
        let len = view_direction.length();
        let forward = view_direction.normalize();
        let right = forward.cross(Vec3::Y).normalize();

        // Create rotation around the right vector (pitch down)
        let rotation = Quat::from_axis_angle(right, -CAMERA_ROTATE_SPEED * dt);
        let new_direction = rotation.mul_vec3(forward).normalize();

        // Prevent camera from flipping by checking if the new direction is not too close to up/down
        if new_direction.dot(Vec3::Y).abs() < 0.95 {
            camera_settings.look_from = camera_settings.look_at + new_direction * len;
            moved = true;
        }
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
    pub center: Vec3,
    pub viewport_height: f32,

    pub viewport_upper_left: Vec3,
    pub viewport_width: f32,

    pub pixel_delta_u: Vec3,
    pub defocus_angle: f32,

    pub pixel_delta_v: Vec3,
    pub aspect_ratio: f32,

    pub defocus_disk_u: Vec3,
    pub _padding0: f32,

    pub viewport_u: Vec3,
    pub _padding1: f32,

    pub defocus_disk_v: Vec3,
    pub max_depth: f32,

    pub look_from: Vec3,
    pub samples_per_pixel: f32,

    pub look_at: Vec3,
    pub camera_has_moved: f32,

    pub vup: Vec3,
    pub random_seed: f32,

    pub viewport_v: Vec3,
    pub defocus_radius: f32,
}

impl From<&CameraSettings> for SceneCamera {
    fn from(settings: &CameraSettings) -> Self {
        let camera = settings;
        let aspect_ratio = crate::SIZE.0 as f32 / crate::SIZE.1 as f32;

        let camera_center = camera.look_from;
        
        let theta = f32::to_radians(camera.field_of_view);
        let h = f32::tan(theta / 2.0);
        let viewport_height = 2.0 * h * camera.focus_distance;
        let viewport_width = viewport_height * aspect_ratio;

        // Calculate viewport vectors
        // Use view direction to calculate viewport orientation
        let w = (camera.look_from - camera.look_at).normalize();
        let u = camera.vup.cross(w).normalize();
        let v = w.cross(u);

        let viewport_u = viewport_width * u;
        let viewport_v = -viewport_height * v; // Negative to flip y-axis

        // Calculate pixel deltas
        let pixel_delta_u = viewport_u / crate::SIZE.0 as f32;
        let pixel_delta_v = viewport_v / crate::SIZE.1 as f32;

        // Calculate viewport upper left corner
        let viewport_upper_left =
            camera_center - (camera.focus_distance * w) - viewport_u / 2.0 - viewport_v / 2.0;

        let defocus_radius =
            camera.focus_distance * f32::tan(f32::to_radians(camera.defocus_angle / 2.0));
        let defocus_disk_u = u * defocus_radius;
        let defocus_disk_v = v * defocus_radius;
        Self {
            center: camera.look_from,
            aspect_ratio,
            viewport_height,
            viewport_width,
            viewport_upper_left,
            pixel_delta_u,
            pixel_delta_v,
            defocus_disk_u,
            defocus_disk_v,
            defocus_angle: camera.defocus_angle,
            look_from: camera.look_from,
            look_at: camera.look_at,
            vup: camera.vup,
            viewport_u,
            viewport_v,
            defocus_radius,
            max_depth: camera.max_depth as f32,
            samples_per_pixel: camera.samples_per_pixel as f32,
            camera_has_moved: if camera.camera_has_moved { 1.0 } else { 0.0 },
            random_seed: rand::random(),
            _padding0: 0.0,
            _padding1: 0.0,
        }
    }
}

// Extract camera settings into the render world
pub fn extract_camera(camera_settings: Res<CameraSettings>, mut commands: Commands) {
    // Convert CameraSettings to the GPU-compatible SceneCamera
    let scene_camera = SceneCamera::from(camera_settings.as_ref());

    // Insert as a resource that will be extracted to the render world
    commands.insert_resource(scene_camera);
}
