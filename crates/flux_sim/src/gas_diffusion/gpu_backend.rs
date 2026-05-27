use std::collections::BTreeMap;
use std::future::Future;
use std::sync::mpsc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use flux_world::{GasLayer, GasMixture, GasPrototypeId, ParticleCount};
use wgpu::util::DeviceExt;

use crate::{GasSimulationBackend, GasStageWorldView, SimError, SimulationBackendId};

const DISPATCH_WORKGROUP_SIZE: u32 = 64;
const GPU_READBACK_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GpuBackendInitError {
    AdapterUnavailable,
    DeviceRequestFailed(String),
}

pub(super) struct GasDiffusionGpuBackend {
    stage_name: &'static str,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    gas_channels: Vec<GasPrototypeId>,
    gas_index_by_id: BTreeMap<GasPrototypeId, usize>,
}

impl GasDiffusionGpuBackend {
    pub(super) fn new(
        stage_name: &'static str,
        mut gas_channels: Vec<GasPrototypeId>,
    ) -> Result<Self, GpuBackendInitError> {
        gas_channels.sort();
        gas_channels.dedup();
        let gas_index_by_id = gas_channels
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, gas)| (gas, index))
            .collect::<BTreeMap<_, _>>();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            // Keep compute backend on the same graphics API family as Bevy on desktop.
            // This avoids cross-backend device initialization paths that are prone to driver/device-loss issues.
            backends: preferred_backends(),
            flags: wgpu::InstanceFlags::empty(),
            backend_options: wgpu::BackendOptions::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        });
        let adapter_result =
            block_on_wgpu(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            }));
        let adapter = match adapter_result {
            Ok(adapter) => adapter,
            Err(_) => return Err(GpuBackendInitError::AdapterUnavailable),
        };

        let device_descriptor = wgpu::DeviceDescriptor {
            label: Some("flux_sim.gas_diffusion.gpu_device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        };
        let device_result = block_on_wgpu(adapter.request_device(&device_descriptor));
        let (device, queue) = match device_result {
            Ok(pair) => pair,
            Err(error) => {
                return Err(GpuBackendInitError::DeviceRequestFailed(error.to_string()));
            }
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("flux_sim.gas_diffusion.gpu_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gas_diffusion.wgsl").into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("flux_sim.gas_diffusion.bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("flux_sim.gas_diffusion.pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("flux_sim.gas_diffusion.compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Ok(Self {
            stage_name,
            device,
            queue,
            pipeline,
            bind_group_layout,
            gas_channels,
            gas_index_by_id,
        })
    }

    fn run_gpu_diffusion(
        &self,
        size: flux_world::GridSize,
        permeability_mask: &[bool],
        previous_dense: &[u32],
        gas_count: usize,
    ) -> Result<Vec<u32>, SimError> {
        let cell_count = usize::try_from(u64::from(size.width) * u64::from(size.height))
            .expect("grid cell count should fit usize");
        let total_scalars =
            cell_count
                .checked_mul(gas_count)
                .ok_or_else(|| SimError::GpuExecutionFailed {
                    stage_name: self.stage_name,
                    reason: "dense gas buffer length overflow".to_owned(),
                })?;
        if previous_dense.len() != total_scalars {
            return Err(SimError::GpuExecutionFailed {
                stage_name: self.stage_name,
                reason: format!(
                    "dense gas input size mismatch: expected={}, actual={}",
                    total_scalars,
                    previous_dense.len()
                ),
            });
        }

        let total_scalars_u32 =
            u32::try_from(total_scalars).map_err(|_| SimError::GpuExecutionFailed {
                stage_name: self.stage_name,
                reason: format!("dense gas scalar count exceeds u32: {}", total_scalars),
            })?;
        let data_size_bytes = u64::from(total_scalars_u32) * 4;
        let aligned_size_bytes = align_to(data_size_bytes, wgpu::MAP_ALIGNMENT);
        let gas_count_u32 = u32::try_from(gas_count).map_err(|_| SimError::GpuExecutionFailed {
            stage_name: self.stage_name,
            reason: format!("gas channel count exceeds u32: {}", gas_count),
        })?;
        let cell_count_u32 =
            u32::try_from(cell_count).map_err(|_| SimError::GpuExecutionFailed {
                stage_name: self.stage_name,
                reason: format!("cell count exceeds u32: {}", cell_count),
            })?;

        let mask_u32 = permeability_mask
            .iter()
            .map(|value| u32::from(*value))
            .collect::<Vec<_>>();
        let params_u32 = [size.width, size.height, gas_count_u32, cell_count_u32];
        let params_bytes = u32_to_le_bytes(&params_u32);
        let mask_bytes = u32_to_le_bytes(&mask_u32);
        let previous_bytes = u32_to_le_bytes(previous_dense);

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("flux_sim.gas_diffusion.params"),
                contents: &params_bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let mask_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("flux_sim.gas_diffusion.permeability_mask"),
                contents: &mask_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            });
        let previous_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("flux_sim.gas_diffusion.previous"),
                contents: &previous_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            });
        let next_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flux_sim.gas_diffusion.next"),
            size: aligned_size_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flux_sim.gas_diffusion.readback"),
            size: aligned_size_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("flux_sim.gas_diffusion.bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: mask_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: previous_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: next_buffer.as_entire_binding(),
                },
            ],
        });

        let workgroup_count = total_scalars_u32.div_ceil(DISPATCH_WORKGROUP_SIZE);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("flux_sim.gas_diffusion.compute_encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("flux_sim.gas_diffusion.compute_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&next_buffer, 0, &staging_buffer, 0, data_size_bytes);
        self.queue.submit([encoder.finish()]);

        let readback = staging_buffer.slice(0..aligned_size_bytes);
        let (sender, receiver) = mpsc::channel::<Result<(), wgpu::BufferAsyncError>>();
        readback.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        self.device
            .poll(wgpu::PollType::wait())
            .map_err(|error| SimError::GpuExecutionFailed {
                stage_name: self.stage_name,
                reason: format!("device poll failed during readback: {error}"),
            })?;
        let map_status = receiver.recv_timeout(GPU_READBACK_TIMEOUT).map_err(|_| {
            SimError::GpuExecutionFailed {
                stage_name: self.stage_name,
                reason: format!("gpu readback timeout after {:?}", GPU_READBACK_TIMEOUT),
            }
        })?;
        map_status.map_err(|error| SimError::GpuExecutionFailed {
            stage_name: self.stage_name,
            reason: format!("gpu readback mapping failed: {error}"),
        })?;

        let mapped = readback.get_mapped_range();
        let output_bytes = mapped
            .get(..usize::try_from(data_size_bytes).expect("readback size should fit usize"))
            .ok_or_else(|| SimError::GpuExecutionFailed {
                stage_name: self.stage_name,
                reason: "gpu readback slice is out of bounds".to_owned(),
            })?;
        let output_dense =
            le_bytes_to_u32(output_bytes).map_err(|reason| SimError::GpuExecutionFailed {
                stage_name: self.stage_name,
                reason,
            })?;
        drop(mapped);
        staging_buffer.unmap();

        Ok(output_dense)
    }
}

fn preferred_backends() -> wgpu::Backends {
    if let Some(from_env) = wgpu::Backends::from_env() {
        return from_env;
    }

    #[cfg(target_os = "windows")]
    {
        // Prefer explicit desktop backends and avoid unstable "all backends" probing.
        // This keeps initialization predictable while still supporting systems without Vulkan.
        wgpu::Backends::DX12 | wgpu::Backends::VULKAN
    }
    #[cfg(target_os = "linux")]
    {
        wgpu::Backends::VULKAN
    }
    #[cfg(target_os = "macos")]
    {
        wgpu::Backends::METAL
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        wgpu::Backends::all()
    }
}

impl GasSimulationBackend for GasDiffusionGpuBackend {
    fn backend_id(&self) -> SimulationBackendId {
        SimulationBackendId::Gpu
    }

    fn execute(
        &self,
        _tick: u64,
        gas_layer: &mut GasLayer,
        world: &GasStageWorldView,
    ) -> Result<(), SimError> {
        let cell_count = world
            .size
            .cell_count()
            .expect("grid size should fit usize for gas stage");
        let permeability_mask = gas_layer.permeability_mask();
        if permeability_mask.len() != cell_count {
            return Err(SimError::GasPermeabilityMaskSizeMismatch {
                expected: cell_count,
                actual: permeability_mask.len(),
            });
        }

        let previous = gas_layer.snapshot();
        let total_before = super::total_particles_by_gas(&previous, permeability_mask);
        if self.gas_channels.is_empty() {
            if total_before.values().any(|particles| *particles > 0) {
                return Err(SimError::GpuExecutionFailed {
                    stage_name: self.stage_name,
                    reason: "gas channel list is empty; configure gas registry channels before initializing GPU backend".to_owned(),
                });
            }
            return Ok(());
        }

        let gas_count = self.gas_channels.len();
        let dense_previous = encode_dense_cells(
            self.stage_name,
            &previous,
            &self.gas_index_by_id,
            cell_count,
            gas_count,
            permeability_mask,
        )?;
        let dense_next =
            self.run_gpu_diffusion(world.size, permeability_mask, &dense_previous, gas_count)?;
        let next_cells = decode_dense_cells(
            &dense_next,
            &self.gas_channels,
            cell_count,
            gas_count,
            permeability_mask,
        );
        let total_after = super::total_particles_by_gas(&next_cells, permeability_mask);
        super::ensure_conservation(&total_before, &total_after)?;
        gas_layer.replace_all(next_cells);
        Ok(())
    }
}

fn encode_dense_cells(
    stage_name: &'static str,
    cells: &[GasMixture],
    index_by_gas: &BTreeMap<GasPrototypeId, usize>,
    cell_count: usize,
    gas_count: usize,
    permeability_mask: &[bool],
) -> Result<Vec<u32>, SimError> {
    let total_scalars =
        cell_count
            .checked_mul(gas_count)
            .ok_or_else(|| SimError::GpuExecutionFailed {
                stage_name,
                reason: "dense gas encode buffer length overflow".to_owned(),
            })?;
    let mut dense = vec![0u32; total_scalars];
    for (cell_index, mixture) in cells.iter().enumerate() {
        if !permeability_mask[cell_index] {
            continue;
        }
        for component in mixture.components() {
            let Some(gas_index) = index_by_gas.get(&component.gas) else {
                return Err(SimError::GpuExecutionFailed {
                    stage_name,
                    reason: format!(
                        "dense gas encode failed: missing channel mapping for gas `{}`",
                        component.gas
                    ),
                });
            };
            let particles_u32 = u32::try_from(component.particles.0).map_err(|_| {
                SimError::GpuParticleCountOverflow {
                    stage_name,
                    gas: component.gas.to_string(),
                    particles: component.particles.0,
                }
            })?;
            let dense_index = cell_index
                .checked_mul(gas_count)
                .and_then(|base| base.checked_add(*gas_index))
                .expect("dense index should be inside preallocated bounds");
            dense[dense_index] = particles_u32;
        }
    }
    Ok(dense)
}

fn decode_dense_cells(
    dense: &[u32],
    gases: &[GasPrototypeId],
    cell_count: usize,
    gas_count: usize,
    permeability_mask: &[bool],
) -> Vec<GasMixture> {
    let mut next = vec![GasMixture::default(); cell_count];
    for cell_index in 0..cell_count {
        if !permeability_mask[cell_index] {
            continue;
        }
        for (gas_index, gas_id) in gases.iter().enumerate() {
            let dense_index = cell_index * gas_count + gas_index;
            let particles = dense[dense_index];
            if particles == 0 {
                continue;
            }
            next[cell_index].set_particles(gas_id.clone(), ParticleCount(u64::from(particles)));
        }
    }
    next
}

fn align_to(value: u64, alignment: u64) -> u64 {
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + alignment - remainder
    }
}

fn u32_to_le_bytes(values: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn le_bytes_to_u32(bytes: &[u8]) -> Result<Vec<u32>, String> {
    if !bytes.len().is_multiple_of(4) {
        return Err(format!(
            "gpu readback bytes length must be multiple of 4, got {}",
            bytes.len()
        ));
    }
    let mut values = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        values.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(values)
}

fn block_on_wgpu<F>(future: F) -> F::Output
where
    F: Future,
{
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}
