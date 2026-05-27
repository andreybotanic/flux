use std::collections::BTreeMap;

use flux_world::{GasComponent, GasMixture, ParticleCount, WorldGrid};

use crate::{
    BackendPolicy, GasSimulationBackend, GasStageWorldView, SimError, SimulationBackendId,
    SimulationStageConfig,
};

const GAS_SIMULATION_STAGE_NAME: &str = "gas";
const GAS_SIMULATION_STAGE_ORDER: u16 = 100;

#[derive(Default)]
pub struct GasDiffusionCpuBackend;

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
        let total_before = total_particles_by_gas(&previous, permeability_mask);
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

        let total_after = total_particles_by_gas(&next, permeability_mask);
        ensure_conservation(&total_before, &total_after)?;

        gas_layer.replace_all(next);
        Ok(())
    }
}

pub struct GasSimulationStage {
    config: SimulationStageConfig,
    cpu_backend: GasDiffusionCpuBackend,
}

impl GasSimulationStage {
    pub fn new(frequency_divider: u64, backend_policy: BackendPolicy) -> Result<Self, SimError> {
        Ok(Self {
            config: SimulationStageConfig::new(
                GAS_SIMULATION_STAGE_NAME,
                GAS_SIMULATION_STAGE_ORDER,
                frequency_divider,
                backend_policy,
            )?,
            cpu_backend: GasDiffusionCpuBackend,
        })
    }

    #[must_use]
    pub fn config(&self) -> &SimulationStageConfig {
        &self.config
    }

    fn resolve_backend(
        &self,
        policy: BackendPolicy,
    ) -> Result<&dyn GasSimulationBackend, SimError> {
        match policy {
            BackendPolicy::CpuOnly => Ok(&self.cpu_backend),
        }
    }

    pub fn execute(&self, tick: u64, world: &mut WorldGrid) -> Result<(), SimError> {
        if !tick.is_multiple_of(self.config.frequency_divider) {
            return Ok(());
        }
        let backend = self
            .resolve_backend(self.config.backend_policy)
            .map_err(|_| SimError::BackendResolutionFailed {
                stage_name: self.config.stage_name,
                backend_policy: self.config.backend_policy,
            })?;
        let world_view = GasStageWorldView { size: world.size() };
        let gas_layer = world.gas_layer_mut();
        backend.execute(tick, gas_layer, &world_view)
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

    use crate::BackendPolicy;

    use super::GasSimulationStage;

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
        let stage = GasSimulationStage::new(1, BackendPolicy::CpuOnly).expect("stage");
        stage.execute(1, &mut world).expect("diffusion should run");

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
        let stage = GasSimulationStage::new(1, BackendPolicy::CpuOnly).expect("stage");
        stage.execute(1, &mut world).expect("diffusion should run");

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
        let stage = GasSimulationStage::new(1, BackendPolicy::CpuOnly).expect("stage");

        let mut world_a = WorldGrid::new(GridSize::new(4, 1)).expect("world");
        let mut world_b = WorldGrid::new(GridSize::new(4, 1)).expect("world");
        world_a
            .set_gas_particles(TilePos::new(1, 0), gas.clone(), ParticleCount(200))
            .expect("seed");
        world_b
            .set_gas_particles(TilePos::new(1, 0), gas.clone(), ParticleCount(200))
            .expect("seed");
        stage.execute(1, &mut world_a).expect("run A");
        stage.execute(1, &mut world_b).expect("run B");

        for x in 0..4 {
            assert_eq!(
                world_a.gas_at(TilePos::new(x, 0)),
                world_b.gas_at(TilePos::new(x, 0))
            );
        }
    }
}
