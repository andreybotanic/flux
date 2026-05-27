use std::collections::BTreeMap;

use flux_world::{GasComponent, GasMixture, ParticleCount};

use crate::{
    BackendPolicy, SimError, SimulationBackendId, SimulationStage, SimulationStageBackend,
    SimulationStageConfig, SimulationStageId, StageExecutionContext,
};

#[derive(Default)]
pub struct GasDiffusionCpuBackend;

impl SimulationStageBackend for GasDiffusionCpuBackend {
    fn backend_id(&self) -> SimulationBackendId {
        SimulationBackendId::Cpu
    }

    fn execute(&self, context: &mut StageExecutionContext<'_>) -> Result<(), SimError> {
        let cell_count = context.world.cell_count();
        if context.gas_permeability_mask.len() != cell_count {
            return Err(SimError::GasPermeabilityMaskSizeMismatch {
                expected: cell_count,
                actual: context.gas_permeability_mask.len(),
            });
        }

        let size = context.world.size();
        let width = usize::try_from(size.width).expect("grid width should fit usize");
        let height = usize::try_from(size.height).expect("grid height should fit usize");

        let previous = context.world.gas_snapshot();
        let total_before = total_particles_by_gas(&previous, context.gas_permeability_mask);
        let mut next = previous.clone();

        for y in 0..height {
            for x in 0..width {
                let index = (y * width) + x;
                if !context.gas_permeability_mask[index] {
                    continue;
                }
                if x + 1 < width {
                    let neighbor = index + 1;
                    if context.gas_permeability_mask[neighbor] {
                        diffuse_between_pair(index, neighbor, &previous, &mut next)?;
                    }
                }
                if y + 1 < height {
                    let neighbor = index + width;
                    if context.gas_permeability_mask[neighbor] {
                        diffuse_between_pair(index, neighbor, &previous, &mut next)?;
                    }
                }
            }
        }

        for (index, permeable) in context.gas_permeability_mask.iter().copied().enumerate() {
            if !permeable {
                next[index].clear_all();
            }
        }

        let total_after = total_particles_by_gas(&next, context.gas_permeability_mask);
        ensure_conservation(&total_before, &total_after)?;

        context
            .world
            .replace_gases_from_snapshot(next)
            .map_err(|source| SimError::GasSnapshotApplyFailed { source })?;
        Ok(())
    }
}

pub struct GasDiffusionStage {
    config: SimulationStageConfig,
    cpu_backend: GasDiffusionCpuBackend,
}

impl GasDiffusionStage {
    pub fn new(frequency_divider: u64, backend_policy: BackendPolicy) -> Result<Self, SimError> {
        Ok(Self {
            config: SimulationStageConfig::new(
                SimulationStageId::GasDiffusion,
                frequency_divider,
                backend_policy,
            )?,
            cpu_backend: GasDiffusionCpuBackend,
        })
    }
}

impl SimulationStage for GasDiffusionStage {
    fn config(&self) -> &SimulationStageConfig {
        &self.config
    }

    fn resolve_backend(
        &self,
        policy: BackendPolicy,
    ) -> Result<&dyn SimulationStageBackend, SimError> {
        match policy {
            BackendPolicy::CpuOnly => Ok(&self.cpu_backend),
        }
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

fn total_particles_by_gas(
    cells: &[GasMixture],
    permeability_mask: &[bool],
) -> BTreeMap<String, u64> {
    let mut totals = BTreeMap::new();
    for (cell_index, mixture) in cells.iter().enumerate() {
        if !permeability_mask[cell_index] {
            continue;
        }
        for component in mixture.components() {
            *totals.entry(component.gas.to_string()).or_insert(0) += component.particles.0;
        }
    }
    totals
}

fn ensure_conservation(
    before: &BTreeMap<String, u64>,
    after: &BTreeMap<String, u64>,
) -> Result<(), SimError> {
    let mut keys = before.keys().cloned().collect::<Vec<_>>();
    for key in after.keys() {
        if !before.contains_key(key) {
            keys.push(key.clone());
        }
    }
    keys.sort();
    keys.dedup();
    for key in keys {
        let before_value = before.get(&key).copied().unwrap_or(0);
        let after_value = after.get(&key).copied().unwrap_or(0);
        if before_value != after_value {
            return Err(SimError::GasConservationViolated {
                gas: key,
                before: before_value,
                after: after_value,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use flux_world::{GasPrototypeId, GridSize, ParticleCount, TilePos, WorldGrid};

    use crate::{BackendPolicy, SimulationStage};

    use super::GasDiffusionStage;

    fn gas_id(value: &str) -> GasPrototypeId {
        GasPrototypeId::parse(value).expect("valid gas id")
    }

    #[test]
    fn diffusion_spreads_to_von_neumann_neighbors_and_keeps_total() {
        let mut world = WorldGrid::new(GridSize::new(3, 3)).expect("world");
        let gas = gas_id("base:gas/oxygen");
        world
            .set_gas_particles(TilePos::new(1, 1), gas.clone(), ParticleCount(400))
            .expect("set center gas");
        let stage = GasDiffusionStage::new(1, BackendPolicy::CpuOnly).expect("stage");
        let permeability_mask = world.build_gas_permeability_mask();
        stage
            .execute(1, &mut world, &permeability_mask)
            .expect("diffusion should run");

        let center = world
            .gas_at(TilePos::new(1, 1))
            .expect("cell")
            .particles_of(&gas)
            .0;
        let left = world
            .gas_at(TilePos::new(0, 1))
            .expect("cell")
            .particles_of(&gas)
            .0;
        let right = world
            .gas_at(TilePos::new(2, 1))
            .expect("cell")
            .particles_of(&gas)
            .0;
        let up = world
            .gas_at(TilePos::new(1, 0))
            .expect("cell")
            .particles_of(&gas)
            .0;
        let down = world
            .gas_at(TilePos::new(1, 2))
            .expect("cell")
            .particles_of(&gas)
            .0;
        let diagonal = world
            .gas_at(TilePos::new(0, 0))
            .expect("cell")
            .particles_of(&gas)
            .0;

        assert!(center < 400, "center should lose particles");
        assert!(left > 0 && right > 0 && up > 0 && down > 0);
        assert_eq!(diagonal, 0, "diagonal cell must stay unchanged");

        let total = [center, left, right, up, down, diagonal]
            .into_iter()
            .sum::<u64>()
            + world
                .gas_at(TilePos::new(2, 0))
                .expect("cell")
                .particles_of(&gas)
                .0
            + world
                .gas_at(TilePos::new(0, 2))
                .expect("cell")
                .particles_of(&gas)
                .0
            + world
                .gas_at(TilePos::new(2, 2))
                .expect("cell")
                .particles_of(&gas)
                .0;
        assert_eq!(total, 400);
    }

    #[test]
    fn solid_cells_are_blocked_and_cleaned_deterministically() {
        let mut world = WorldGrid::new(GridSize::new(2, 1)).expect("world");
        let gas = gas_id("base:gas/oxygen");
        let solid = gas_id("base:solid_cell/floor_cell");
        world
            .set_solid_cell_at(TilePos::new(0, 0), Some(solid))
            .expect("set solid");
        world
            .set_gas_particles(TilePos::new(0, 0), gas.clone(), ParticleCount(80))
            .expect("inject invalid gas");
        world
            .set_gas_particles(TilePos::new(1, 0), gas.clone(), ParticleCount(120))
            .expect("set valid gas");
        let stage = GasDiffusionStage::new(1, BackendPolicy::CpuOnly).expect("stage");
        let permeability_mask = world.build_gas_permeability_mask();
        stage
            .execute(1, &mut world, &permeability_mask)
            .expect("diffusion should run");

        assert_eq!(
            world
                .gas_at(TilePos::new(0, 0))
                .expect("cell")
                .particles_of(&gas),
            ParticleCount(0)
        );
        assert_eq!(
            world
                .gas_at(TilePos::new(1, 0))
                .expect("cell")
                .particles_of(&gas),
            ParticleCount(120)
        );
    }

    #[test]
    fn same_input_produces_same_output() {
        let gas = gas_id("base:gas/oxygen");
        let stage = GasDiffusionStage::new(1, BackendPolicy::CpuOnly).expect("stage");

        let mut world_a = WorldGrid::new(GridSize::new(4, 1)).expect("world");
        let mut world_b = WorldGrid::new(GridSize::new(4, 1)).expect("world");
        world_a
            .set_gas_particles(TilePos::new(1, 0), gas.clone(), ParticleCount(200))
            .expect("seed");
        world_b
            .set_gas_particles(TilePos::new(1, 0), gas.clone(), ParticleCount(200))
            .expect("seed");
        let mask_a = world_a.build_gas_permeability_mask();
        let mask_b = world_b.build_gas_permeability_mask();
        stage.execute(1, &mut world_a, &mask_a).expect("run A");
        stage.execute(1, &mut world_b, &mask_b).expect("run B");

        for x in 0..4 {
            assert_eq!(
                world_a.gas_at(TilePos::new(x, 0)),
                world_b.gas_at(TilePos::new(x, 0))
            );
        }
    }
}
