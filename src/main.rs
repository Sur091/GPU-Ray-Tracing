use gpu_ray_tracing::run;

fn main() {
    run();
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
struct CustomMaterial {
    #[uniform(0)]
    color: LinearRgba,
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}