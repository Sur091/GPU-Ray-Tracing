//! A compute shader that simulates Conway's Game of Life.
//!
//! Compute shaders use the GPU for computing arbitrary information, that may be independent of what
//! is rendered to the screen.

use crate::scene::camera::SceneCamera;
use bevy::{
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
};
use std::borrow::Cow;

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "compute_shader_example.wgsl";

// const DISPLAY_FACTOR: u32 = 4;
// const SIZE: (u32, u32) = (1280 / DISPLAY_FACTOR, 720 / DISPLAY_FACTOR);
const WORKGROUP_SIZE: u32 = 8;

pub fn setup_compute_shader(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new_fill(
        Extent3d {
            width: crate::WINDOW_SIZE.0,
            height: crate::WINDOW_SIZE.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let image0 = images.add(image.clone());
    let image1 = images.add(image);

    commands.spawn((
        Sprite {
            image: image0.clone(),
            custom_size: Some(Vec2::new(
                crate::WINDOW_SIZE.0 as f32,
                crate::WINDOW_SIZE.1 as f32,
            )),
            ..default()
        },
        Transform::default(),
    ));
    commands.spawn(Camera2d);

    commands.insert_resource(ComputeShaderImages {
        texture_a: image0,
        texture_b: image1,
    });

    commands.insert_resource(SceneCamera::default());
}

// Switch texture to display every frame to show the one that was written to most recently.
pub fn switch_textures(images: Res<ComputeShaderImages>, mut sprite: Single<&mut Sprite>) {
    if sprite.image == images.texture_a {
        sprite.image = images.texture_b.clone_weak();
    } else {
        sprite.image = images.texture_a.clone_weak();
    }
}

pub struct ComputeShaderPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ComputeShaderLabel;

impl Plugin for ComputeShaderPlugin {
    fn build(&self, app: &mut App) {
        // Extract the game of life image resource from the main world into the render world
        // for operation on by the compute shader and display on the sprite.
        app.add_plugins((
            ExtractResourcePlugin::<ComputeShaderImages>::default(),
            ExtractResourcePlugin::<SceneCamera>::default(),
        ));
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(ComputeShaderLabel, ComputeShaderNode::default());
        render_graph.add_node_edge(ComputeShaderLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<ComputeShaderPipeline>();
    }
}

#[derive(Resource, Clone, ExtractResource)]
pub struct ComputeShaderImages {
    texture_a: Handle<Image>,
    texture_b: Handle<Image>,
}

#[derive(Resource)]
struct ComputeShaderImageBindGroups([BindGroup; 2]);

#[derive(Resource)]
struct CameraBindGroup(BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<ComputeShaderPipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    compute_shader_images: Res<ComputeShaderImages>,
    render_device: Res<RenderDevice>,
    scene_camera: Res<SceneCamera>,
) {
    let view_a = gpu_images.get(&compute_shader_images.texture_a).unwrap();
    let view_b = gpu_images.get(&compute_shader_images.texture_b).unwrap();

    let camera_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[scene_camera.clone()]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let camera_bind_group = render_device.create_bind_group(
        None,
        &pipeline.camera_bind_group_layout,
        &[BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    );

    let bind_group_0 = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((&view_a.texture_view, &view_b.texture_view)),
    );
    let bind_group_1 = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((&view_b.texture_view, &view_a.texture_view)),
    );
    commands.insert_resource(ComputeShaderImageBindGroups([bind_group_0, bind_group_1]));
    commands.insert_resource(CameraBindGroup(camera_bind_group));
}

#[derive(Resource)]
struct ComputeShaderPipeline {
    texture_bind_group_layout: BindGroupLayout,
    camera_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for ComputeShaderPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group_layout = render_device.create_bind_group_layout(
            "ComputeShaderImages",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::ReadOnly),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                ),
            ),
        );
        let camera_bind_group_layout = render_device.create_bind_group_layout(
            "ComputeShaderCamera",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (uniform_buffer::<SceneCamera>(false),),
            ),
        );
        let shader = world.load_asset(SHADER_ASSET_PATH);
        let pipeline_cache = world.resource::<PipelineCache>();
        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![
                texture_bind_group_layout.clone(),
                camera_bind_group_layout.clone(),
            ],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init"),
            zero_initialize_workgroup_memory: false,
        });
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![
                texture_bind_group_layout.clone(),
                camera_bind_group_layout.clone(),
            ],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
            zero_initialize_workgroup_memory: false,
        });

        ComputeShaderPipeline {
            texture_bind_group_layout,
            camera_bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

enum ComputeShaderState {
    Loading,
    Init,
    Update(usize),
}

struct ComputeShaderNode {
    state: ComputeShaderState,
}

impl Default for ComputeShaderNode {
    fn default() -> Self {
        Self {
            state: ComputeShaderState::Loading,
        }
    }
}

impl render_graph::Node for ComputeShaderNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<ComputeShaderPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            ComputeShaderState::Loading => {
                match pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline) {
                    CachedPipelineState::Ok(_) => {
                        self.state = ComputeShaderState::Init;
                    }
                    CachedPipelineState::Err(err) => {
                        panic!("Initializing assets/{SHADER_ASSET_PATH}:\n{err}")
                    }
                    _ => {}
                }
            }
            ComputeShaderState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = ComputeShaderState::Update(1);
                }
            }
            ComputeShaderState::Update(0) => {
                self.state = ComputeShaderState::Update(1);
            }
            ComputeShaderState::Update(1) => {
                self.state = ComputeShaderState::Update(0);
            }
            ComputeShaderState::Update(_) => unreachable!(),
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let bind_groups = &world.resource::<ComputeShaderImageBindGroups>().0;
        let camera_bind_group = &world.resource::<CameraBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<ComputeShaderPipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        // select the pipeline based on the current state
        match self.state {
            ComputeShaderState::Loading => {}
            ComputeShaderState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_bind_group(0, &bind_groups[0], &[]);
                pass.set_bind_group(1, camera_bind_group, &[]);
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(
                    crate::WINDOW_SIZE.0 / WORKGROUP_SIZE,
                    crate::WINDOW_SIZE.1 / WORKGROUP_SIZE,
                    1,
                );
            }
            ComputeShaderState::Update(index) => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_bind_group(0, &bind_groups[index], &[]);
                pass.set_bind_group(1, camera_bind_group, &[]);
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(
                    crate::WINDOW_SIZE.0 / WORKGROUP_SIZE,
                    crate::WINDOW_SIZE.1 / WORKGROUP_SIZE,
                    1,
                );
            }
        }

        Ok(())
    }
}
