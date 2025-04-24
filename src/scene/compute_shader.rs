use bevy::{
    asset::load_internal_asset,
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{
            binding_types::{texture_storage_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        Render, RenderApp, RenderSet,
    },
    // Import necessary sprite components for Bevy 0.15
    sprite::{Material2d, Material2dPlugin},
};

use crate::scene::camera::SceneCamera;

use std::borrow::Cow;

/// Asset path for the compute shader.
// const SHADER_ASSET_PATH: &str = "compute_shader_example.wgsl";

/// Handle for the internal post-processing shader asset.
const POST_PROCESS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(1420694201_u128.wrapping_add(1)); // Ensure uniqueness
/// Handle for the internal custom vertex shader asset.
const POST_PROCESS_VERTEX_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(1420694201_u128.wrapping_add(2)); // Ensure uniqueness
/// Handle for the internal compute shader asset.
const COMPUTE_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(1420694201_u128.wrapping_add(3)); // Ensure uniqueness

/// Workgroup size for the compute shader (must match the shader!).
const WORKGROUP_SIZE: u32 = 8;

// --- Plugin Setup ---

pub struct ComputeShaderPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ComputeShaderLabel;

impl Plugin for ComputeShaderPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            POST_PROCESS_SHADER_HANDLE,
            "post_process.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            POST_PROCESS_VERTEX_SHADER_HANDLE,
            "post_process_vertex.wgsl", // Path relative to internal asset loading root
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            COMPUTE_SHADER_HANDLE,
            "compute_shader_example.wgsl", // Load compute shader as internal asset
            Shader::from_wgsl
        );

        app.add_plugins(Material2dPlugin::<PostProcessMaterial>::default())
            .add_plugins((
                ExtractResourcePlugin::<ComputeShaderImages>::default(),
                ExtractResourcePlugin::<SceneCamera>::default(),
            ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            // Initialize the resource using its Default implementation
            .init_resource::<ComputeShaderPipeline>()
            // Add the system to create layouts and queue the pipeline compilation
            .add_systems(Render, prepare_compute_pipelines.in_set(RenderSet::Prepare))
            // Add the system to create bind groups (depends on pipeline layouts)
            .add_systems(
                Render,
                prepare_bind_groups.in_set(RenderSet::PrepareBindGroups),
            );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(ComputeShaderLabel, ComputeShaderNode::default());
        render_graph.add_node_edge(ComputeShaderLabel, bevy::render::graph::CameraDriverLabel);
    }
}

// --- Material Definition ---
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct PostProcessMaterial {
    // Explicitly set binding group 1 for texture and sampler
    #[texture(0)]
    #[sampler(1)]
    source_image: Handle<Image>,
}
impl Material2d for PostProcessMaterial {
    fn fragment_shader() -> ShaderRef {
        POST_PROCESS_SHADER_HANDLE.into()
    }
    fn vertex_shader() -> ShaderRef {
        POST_PROCESS_VERTEX_SHADER_HANDLE.into() // Use our custom vertex shader
    }
}

// --- Resources ---
#[derive(Resource, Clone, ExtractResource)]
pub struct ComputeShaderImages {
    texture_a: Handle<Image>,
    texture_b: Handle<Image>,
}
#[derive(Resource)]
struct ComputeShaderImageBindGroups([BindGroup; 2]);
#[derive(Resource)]
struct CameraUniformBindGroup(BindGroup);

/// Holds the compute pipeline layout and optional ID. Initialized via Default.
#[derive(Resource, Default)] // Add Default derive
struct ComputeShaderPipeline {
    texture_bind_group_layout: Option<BindGroupLayout>, // Use Option
    camera_bind_group_layout: Option<BindGroupLayout>,  // Use Option
    update_pipeline: Option<CachedComputePipelineId>,   // Use Option
}

// --- Systems ---

/// Creates the initial textures, compute shader images resource,
/// and the 2D mesh entity for displaying the result.
pub fn setup_compute_shader(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PostProcessMaterial>>,
) {
    // Create the first texture (Texture A)
    let mut image = Image::new_fill(
        Extent3d {
            width: crate::WINDOW_SIZE.0,
            height: crate::WINDOW_SIZE.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 16],                        // 4x f32 = 16 bytes, initialized to zeros
        TextureFormat::Rgba32Float,      // Crucial: Use float format for accumulation
        RenderAssetUsages::RENDER_WORLD, // Available to the render world
    );
    // Set required usages for compute shader read/write and texture binding
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let image_a_handle = images.add(image.clone()); // Clone for the second handle

    // Create the second texture (Texture B)
    let image_b_handle = images.add(image);

    // Store handles in a resource for easy access
    commands.insert_resource(ComputeShaderImages {
        texture_a: image_a_handle.clone(), // Clone for the resource
        texture_b: image_b_handle,
    });

    // Create the mesh asset (a simple quad covering the screen/window)
    let quad_mesh_handle = meshes.add(Rectangle::from_size(Vec2::new(
        crate::WINDOW_SIZE.0 as f32,
        crate::WINDOW_SIZE.1 as f32,
    )));

    // Create the material asset, initially pointing to Texture A
    let material_handle = materials.add(PostProcessMaterial {
        source_image: image_a_handle, // Start displaying texture A
    });

    // Spawn the 2D entity using individual components as MaterialMesh2dBundle is deprecated
    commands.spawn((
        Mesh2d(quad_mesh_handle), // Use the explicit wrapper struct Mesh2dHandle
        MeshMaterial2d(material_handle), // Add ViewVisibility
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)), // Add Transform
    ));

    // Add a 2D camera to see the quad
    commands.spawn(Camera2d::default()); // Use Camera2dBundle for clarity
}

/// Swaps the `source_image` handle in the `PostProcessMaterial` asset
/// Swaps the `source_image` handle in the `PostProcessMaterial` assets.
/// Note: This iterates through *all* loaded assets of this type.
/// If multiple materials exist, this will swap textures in all of them.
pub fn switch_textures(
    images: Res<ComputeShaderImages>,
    mut materials: ResMut<Assets<PostProcessMaterial>>,
) {
    // Iterate through all loaded PostProcessMaterial assets
    for (_id, material_asset) in materials.iter_mut() {
        // Swap the source image handle
        if material_asset.source_image == images.texture_a {
            material_asset.source_image = images.texture_b.clone_weak();
            // info!("Switched post-process texture to B for asset");
        } else {
            material_asset.source_image = images.texture_a.clone_weak();
            // info!("Switched post-process texture to A for asset");
        }
        // If you only want to modify ONE specific material, you'd need its Handle
        // and use materials.get_mut(specific_handle) instead of iterating.
    }
}

/// System that runs in the Render schedule to create layouts and queue the compute pipeline.
fn prepare_compute_pipelines(
    mut pipeline_res: ResMut<ComputeShaderPipeline>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    _asset_server: Res<AssetServer>,
) {
    let mut layouts_created_this_call = false;

    // Create texture layout if it doesn't exist
    if pipeline_res.texture_bind_group_layout.is_none() {
        let layout = render_device.create_bind_group_layout(
            Some("Compute Shader Images Layout"),
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_storage_2d(TextureFormat::Rgba32Float, StorageTextureAccess::ReadOnly),
                    texture_storage_2d(TextureFormat::Rgba32Float, StorageTextureAccess::WriteOnly),
                ),
            ),
        );
        pipeline_res.texture_bind_group_layout = Some(layout);
        layouts_created_this_call = true;
        debug!("Texture bind group layout created.");
    }

    // Create camera layout if it doesn't exist
    if pipeline_res.camera_bind_group_layout.is_none() {
        let layout = render_device.create_bind_group_layout(
            Some("SceneCamera Layout"),
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                // Ensure SceneCamera is available for type check
                (uniform_buffer::<SceneCamera>(false),),
            ),
        );
        pipeline_res.camera_bind_group_layout = Some(layout);
        layouts_created_this_call = true;
        debug!("Camera bind group layout created.");
    }

    // Avoid queueing the pipeline in the same frame layouts were created,
    // as other systems might need them immediately.
    if layouts_created_this_call {
        debug!("Layouts created, delaying pipeline queueing until next frame.");
        return;
    }

    // Queue the pipeline only if layouts exist and pipeline ID is None
    if pipeline_res.update_pipeline.is_none() {
        // Use Option::zip to proceed only if both layouts are Some
        if let Some((tex_layout, cam_layout)) = pipeline_res
            .texture_bind_group_layout
            .as_ref()
            .zip(pipeline_res.camera_bind_group_layout.as_ref())
        {
            let shader: Handle<Shader> = COMPUTE_SHADER_HANDLE;
            let layout_vec = vec![tex_layout.clone(), cam_layout.clone()];

            let pipeline_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("Update Pipeline".into()),
                layout: layout_vec,
                shader,
                shader_defs: vec![],
                entry_point: Cow::from("update"),
                push_constant_ranges: Vec::new(),
                zero_initialize_workgroup_memory: false,
            });

            pipeline_res.update_pipeline = Some(pipeline_id);
            info!("Compute pipeline queued for compilation.");
        } else {
            // Should not happen if logic above is correct
            error!("Layouts were expected but missing in prepare_compute_pipelines!");
        }
    }
}

/// Creates the bind groups required by the compute shader in the render world.
fn prepare_bind_groups(
    mut commands: Commands,
    pipeline: Res<ComputeShaderPipeline>, // Use immutable reference here
    gpu_images: Res<RenderAssets<GpuImage>>,
    compute_shader_images: Res<ComputeShaderImages>,
    render_device: Res<RenderDevice>,
    scene_camera_uniform: Res<SceneCamera>,
) {
    // Get the layouts; return early if they aren't created yet.
    let Some(texture_layout) = pipeline.texture_bind_group_layout.as_ref() else {
        // Don't log every frame, maybe only once? Or use trace level
        // debug!("Texture layout not ready for bind group creation.");
        return;
    };
    let Some(camera_layout) = pipeline.camera_bind_group_layout.as_ref() else {
        // debug!("Camera layout not ready for bind group creation.");
        return;
    };

    // Get texture views. Guard against assets not being loaded yet.
    let Some(view_a_gpu) = gpu_images.get(&compute_shader_images.texture_a) else {
        return;
    };
    let Some(view_b_gpu) = gpu_images.get(&compute_shader_images.texture_b) else {
        return;
    };
    let view_a = &view_a_gpu.texture_view;
    let view_b = &view_b_gpu.texture_view;

    // Create camera uniform buffer.
    let camera_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("SceneCamera Uniform Buffer"),
        contents: bytemuck::bytes_of(scene_camera_uniform.as_ref()),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    // Create camera bind group using the existing layout.
    let camera_bind_group = render_device.create_bind_group(
        Some("SceneCamera Bind Group"),
        camera_layout, // Use the obtained layout
        &[BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    );
    // Insert or update the resource. Use insert here as it runs every frame needed.
    commands.insert_resource(CameraUniformBindGroup(camera_bind_group));

    // Create texture bind groups using the existing layout.
    let bind_group_0 = render_device.create_bind_group(
        Some("Compute Shader Images Bind Group 0 (A->B)"),
        texture_layout, // Use the obtained layout
        &BindGroupEntries::sequential((view_a, view_b)),
    );
    let bind_group_1 = render_device.create_bind_group(
        Some("Compute Shader Images Bind Group 1 (B->A)"),
        texture_layout, // Use the obtained layout
        &BindGroupEntries::sequential((view_b, view_a)),
    );
    // Insert or update the resource.
    commands.insert_resource(ComputeShaderImageBindGroups([bind_group_0, bind_group_1]));
}

// --- Compute Pipeline Setup ---
// DELETE THE FromWorld impl entirely

// --- Render Graph Node ---
#[derive(Default)]
enum ComputeShaderState {
    #[default]
    Loading,
    Update(usize),
}
#[derive(Default)]
struct ComputeShaderNode {
    state: ComputeShaderState,
}
impl render_graph::Node for ComputeShaderNode {
    fn update(&mut self, world: &mut World) {
        /* ... unchanged ... */
        let pipeline_res = world.resource::<ComputeShaderPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        if world.get_resource::<CameraUniformBindGroup>().is_none()
            || world
                .get_resource::<ComputeShaderImageBindGroups>()
                .is_none()
        // Also check texture groups
        {
            self.state = ComputeShaderState::Loading;
            return;
        }

        // Check if the pipeline ID exists in the resource yet
        let Some(pipeline_id) = pipeline_res.update_pipeline else {
            // Pipeline hasn't been queued by the prepare system yet
            self.state = ComputeShaderState::Loading;
            return;
        };

        match self.state {
            ComputeShaderState::Loading => {
                // Check if the *queued* pipeline has finished compiling.
                match pipeline_cache.get_compute_pipeline_state(pipeline_id) {
                    CachedPipelineState::Ok(_) => {
                        self.state = ComputeShaderState::Update(0);
                        info!("Compute pipeline ready, starting progressive rendering.");
                    }
                    CachedPipelineState::Err(err) => {
                        error!("Compute shader pipeline failed to compile: {err}");
                        self.state = ComputeShaderState::Loading;
                    }
                    _ => {
                        // Still compiling
                        self.state = ComputeShaderState::Loading;
                    }
                }
            }
            ComputeShaderState::Update(index) => {
                // Toggle the index for the next frame's bind group.
                self.state = ComputeShaderState::Update(1 - index);
            }
        }
    }
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        /* ... unchanged ... */
        let texture_bind_groups = world.resource::<ComputeShaderImageBindGroups>();
        let camera_bind_group = world.resource::<CameraUniformBindGroup>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let compute_pipeline_res = world.resource::<ComputeShaderPipeline>(); // Renamed to avoid conflict

        // Get the pipeline ID *from the resource*
        let Some(pipeline_id) = compute_pipeline_res.update_pipeline else {
            debug!("Compute pipeline ID not available yet, skipping dispatch.");
            return Ok(()); // Not an error, just waiting
        };

        // Get the compiled pipeline using the ID
        let Some(update_pipeline) = pipeline_cache.get_compute_pipeline(pipeline_id) else {
            debug!("Compute pipeline not compiled yet, skipping dispatch.");
            return Ok(()); // Not an error, just waiting
        };

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(1, &camera_bind_group.0, &[]); // Set camera group

        match self.state {
            ComputeShaderState::Loading => {
                // Should ideally not be in Loading state if checks in update passed,
                // but do nothing just in case.
            }
            ComputeShaderState::Update(index) => {
                pass.set_pipeline(update_pipeline); // Set the obtained pipeline
                pass.set_bind_group(0, &texture_bind_groups.0[index], &[]); // Set texture group
                pass.dispatch_workgroups(
                    (crate::WINDOW_SIZE.0 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
                    (crate::WINDOW_SIZE.1 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
                    1,
                );
            }
        }

        Ok(())
    }
}
