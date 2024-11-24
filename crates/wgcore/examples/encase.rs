use nalgebra::{DVector, Vector4};
use wgcore::composer::ComposerExt;
use wgcore::gpu::GpuInstance;
use wgcore::kernel::{KernelInvocationBuilder, KernelInvocationQueue};
use wgcore::tensor::GpuVector;
use wgcore::Shader;
use wgpu::{BufferUsages, ComputePipeline};

#[derive(Copy, Clone, PartialEq, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct BytemuckStruct {
    value: f32,
}

#[derive(Copy, Clone, PartialEq, Debug, Default, encase::ShaderType)]
#[repr(C)]
pub struct EncaseStruct {
    value: f32,
    // This implies some internal padding, so we can’t rely on bytemuck.
    // Encase will handle that properly.
    value2: Vector4<f32>,
}

#[derive(Shader)]
#[shader(
    src = "encase.wgsl",
    composable = false
)]
struct ShaderEncase {
    main: ComputePipeline
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the gpu device and its queue.
    //
    // Note that `GpuInstance` is just a simple helper struct for initializing the gpu resources.
    // You are free to initialize them independently if more control is needed, or reuse the ones
    // that were already created/owned by e.g., a game engine.
    let gpu = GpuInstance::new().await?;

    // Load and compile our kernel. The `from_device` function was generated by the `Shader` derive.
    // Note that its dependency to `Composable` is automatically resolved by the `Shader` derive
    // too.
    let kernel = ShaderEncase::from_device(gpu.device())?;

    // Create the buffers.
    const LEN: u32 = 1000;
    let a_data = (0..LEN).map(|x| EncaseStruct { value: x as f32, value2: Vector4::repeat(x as f32 * 10.0)}).collect::<Vec<_>>();
    let b_data = (0..LEN).map(|x| BytemuckStruct { value: x as f32}).collect::<Vec<_>>();
    // Call `encase` instead of `init` because `EncaseStruct` isn’t `Pod`.
    // The `encase` function has a bit of overhead so bytemuck should be preferred whenever possible.
    let a_buf = GpuVector::encase(gpu.device(), &a_data, BufferUsages::STORAGE);
    let b_buf = GpuVector::init(gpu.device(), &b_data, BufferUsages::STORAGE);

    // Queue the operation.
    let mut queue = KernelInvocationQueue::new(gpu.device());
    KernelInvocationBuilder::new(&mut queue, &kernel.main)
        .bind0([a_buf.buffer(), b_buf.buffer()])
        .queue(LEN.div_ceil(64));

    // Encode & submit the operation to the gpu.
    let mut encoder = gpu.device().create_command_encoder(&Default::default());
    queue.encode(&mut encoder, None);
    gpu.queue().submit(Some(encoder.finish()));

    Ok(())
}