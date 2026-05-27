use flux_world::{GasComponent, GasMixture, ParticleCount};

use crate::{GasSimulationBackend, GasStageWorldView, SimError, SimulationBackendId};

#[derive(Default)]
pub(super) struct GasDiffusionCpuBackend;

impl GasSimulationBackend for GasDiffusionCpuBackend {
    fn backend_id(&self) -> SimulationBackendId {
        SimulationBackendId::Cpu
    }

    fn execute(
        &self,
        _tick: u64,
        gas_layer: &mut flux_world::GasLayer,
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

        let width = usize::try_from(world.size.width).expect("grid width should fit usize");
        let height = usize::try_from(world.size.height).expect("grid height should fit usize");

        let previous = gas_layer.snapshot();
        let total_before = super::total_particles_by_gas(&previous, permeability_mask);
        let mut next = previous.clone();

        for y in 0..height {
            for x in 0..width {
                let index = (y * width) + x;
                if !permeability_mask[index] {
                    continue;
                }
                if x + 1 < width {
                    let neighbor = index + 1;
                    if permeability_mask[neighbor] {
                        diffuse_between_pair(index, neighbor, &previous, &mut next)?;
                    }
                }
                if y + 1 < height {
                    let neighbor = index + width;
                    if permeability_mask[neighbor] {
                        diffuse_between_pair(index, neighbor, &previous, &mut next)?;
                    }
                }
            }
        }

        for (index, permeable) in permeability_mask.iter().copied().enumerate() {
            if !permeable {
                next[index].clear_all();
            }
        }

        let total_after = super::total_particles_by_gas(&next, permeability_mask);
        super::ensure_conservation(&total_before, &total_after)?;

        gas_layer.replace_all(next);
        Ok(())
    }
}

fn diffuse_between_pair(
    left_index: usize,
    right_index: usize,
    previous: &[GasMixture],
    next: &mut [GasMixture],
) -> Result<(), SimError> {
    for_each_component_union(
        previous[left_index].components(),
        previous[right_index].components(),
        |gas, left_particles, right_particles| {
            if left_particles == right_particles {
                return Ok(());
            }
            let transfer = left_particles.abs_diff(right_particles) / 4;
            if transfer == 0 {
                return Ok(());
            }

            if left_particles > right_particles {
                next[left_index]
                    .remove_particles(gas.clone(), ParticleCount(transfer))
                    .map_err(|source| SimError::GasParticleOpFailed { source })?;
                next[right_index]
                    .add_particles(gas.clone(), ParticleCount(transfer))
                    .map_err(|source| SimError::GasParticleOpFailed { source })?;
            } else {
                next[right_index]
                    .remove_particles(gas.clone(), ParticleCount(transfer))
                    .map_err(|source| SimError::GasParticleOpFailed { source })?;
                next[left_index]
                    .add_particles(gas.clone(), ParticleCount(transfer))
                    .map_err(|source| SimError::GasParticleOpFailed { source })?;
            }
            Ok(())
        },
    )
}

fn for_each_component_union<F>(
    left: &[GasComponent],
    right: &[GasComponent],
    mut visit: F,
) -> Result<(), SimError>
where
    F: FnMut(&flux_world::GasPrototypeId, u64, u64) -> Result<(), SimError>,
{
    let mut left_index = 0usize;
    let mut right_index = 0usize;
    while left_index < left.len() || right_index < right.len() {
        match (left.get(left_index), right.get(right_index)) {
            (Some(left_component), Some(right_component)) => {
                if left_component.gas == right_component.gas {
                    visit(
                        &left_component.gas,
                        left_component.particles.0,
                        right_component.particles.0,
                    )?;
                    left_index += 1;
                    right_index += 1;
                } else if left_component.gas < right_component.gas {
                    visit(&left_component.gas, left_component.particles.0, 0)?;
                    left_index += 1;
                } else {
                    visit(&right_component.gas, 0, right_component.particles.0)?;
                    right_index += 1;
                }
            }
            (Some(left_component), None) => {
                visit(&left_component.gas, left_component.particles.0, 0)?;
                left_index += 1;
            }
            (None, Some(right_component)) => {
                visit(&right_component.gas, 0, right_component.particles.0)?;
                right_index += 1;
            }
            (None, None) => break,
        }
    }
    Ok(())
}
