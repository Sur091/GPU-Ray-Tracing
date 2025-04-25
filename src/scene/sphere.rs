use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::ShaderType,
    },
};
use bytemuck::{Pod, Zeroable};

// Number of spheres to send to the GPU
pub const MAX_SPHERES: usize = 100;

// GPU-compatible sphere and material definitions
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, ShaderType)]
pub struct GpuMaterial {
    pub color: Vec4,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, ShaderType)]
pub struct GpuSphere {
    pub position: Vec3,
    pub radius: f32,
    pub material: GpuMaterial,
}

// Resource to hold all spheres in the scene
#[derive(Resource, Clone, ExtractResource)]
pub struct SphereCollection {
    pub spheres: Vec<GpuSphere>,
    pub count: u32,
}

impl Default for SphereCollection {
    fn default() -> Self {
        Self {
            spheres: Vec::with_capacity(MAX_SPHERES),
            count: 0,
        }
    }
}

// Helper function to create a basic collection of spheres
pub fn create_default_spheres() -> SphereCollection {
    let mut collection = SphereCollection::default();

    // Add a ground sphere
    collection.spheres.push(GpuSphere {
        position: Vec3::new(0.0, -1000.0, 0.0),
        radius: 1000.0,
        material: GpuMaterial {
            color: Vec4::new(0.5, 0.5, 0.5, -2.0), // Ground material (diffuse)
        },
    });

    // Add random smaller spheres
    // let mut rng = rand::thread_rng();
    for a in -7..7 {
        for b in -7..7 {
            let choose_mat = rand::random::<f32>();
            let center = Vec3::new(
                a as f32 + 0.9 * rand::random::<f32>(),
                0.2,
                b as f32 + 0.9 * rand::random::<f32>(),
            );

            // Don't place spheres too close to the main spheres
            if (center - Vec3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                if choose_mat < 0.8 {
                    // Diffuse material
                    let albedo = Vec3::new(
                        rand::random::<f32>() * rand::random::<f32>(),
                        rand::random::<f32>() * rand::random::<f32>(),
                        rand::random::<f32>() * rand::random::<f32>(),
                    );
                    collection.spheres.push(GpuSphere {
                        position: center,
                        radius: 0.2,
                        material: GpuMaterial {
                            color: Vec4::new(albedo.x, albedo.y, albedo.z, -2.0), // Diffuse
                        },
                    });
                } else if choose_mat < 0.95 {
                    // Metal material
                    let albedo = Vec3::new(
                        0.5 * (1.0 + rand::random::<f32>()),
                        0.5 * (1.0 + rand::random::<f32>()),
                        0.5 * (1.0 + rand::random::<f32>()),
                    );
                    let fuzz = 0.5 * rand::random::<f32>();
                    collection.spheres.push(GpuSphere {
                        position: center,
                        radius: 0.2,
                        material: GpuMaterial {
                            color: Vec4::new(albedo.x, albedo.y, albedo.z, fuzz), // Metal with fuzz
                        },
                    });
                } else {
                    // Glass material
                    collection.spheres.push(GpuSphere {
                        position: center,
                        radius: 0.2,
                        material: GpuMaterial {
                            color: Vec4::new(1.5, 0.0, 0.0, 2.0), // Glass (refractive index 1.5)
                        },
                    });
                }
            }
        }
    }

    // Add a few special spheres
    collection.spheres.push(GpuSphere {
        position: Vec3::new(0.0, 1.0, 0.0),
        radius: 1.0,
        material: GpuMaterial {
            color: Vec4::new(1.5, 0.0, 0.0, 2.0), // Glass
        },
    });

    collection.spheres.push(GpuSphere {
        position: Vec3::new(-4.0, 1.0, 0.0),
        radius: 1.0,
        material: GpuMaterial {
            color: Vec4::new(0.4, 0.2, 0.1, -2.0), // Diffuse
        },
    });

    collection.spheres.push(GpuSphere {
        position: Vec3::new(4.0, 1.0, 0.0),
        radius: 1.0,
        material: GpuMaterial {
            color: Vec4::new(0.7, 0.6, 0.5, 0.0), // Metal
        },
    });

    // Set the actual count
    collection.count = collection.spheres.len() as u32;

    // Fill remaining slots with dummy spheres if needed
    while collection.spheres.len() < MAX_SPHERES {
        collection.spheres.push(GpuSphere {
            position: Vec3::ZERO,
            radius: 0.0,
            material: GpuMaterial {
                color: Vec4::ZERO,
            },
        });
    }

    collection
}

// Plugin to handle sphere setup and extraction
pub struct SpheresPlugin;

impl Plugin for SpheresPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SphereCollection>()
            .add_systems(Startup, setup_spheres)
            .add_plugins(ExtractResourcePlugin::<SphereCollection>::default());
    }
}

// Initialize the sphere collection at startup
fn setup_spheres(mut commands: Commands) {
    let spheres = create_default_spheres();
    commands.insert_resource(spheres);
}
  
