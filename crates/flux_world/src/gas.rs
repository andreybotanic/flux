use crate::{GasMixtureError, GasPrototypeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct ParticleCount(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GasComponent {
    pub gas: GasPrototypeId,
    pub particles: ParticleCount,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GasMixture {
    components: Vec<GasComponent>,
}

impl GasMixture {
    #[must_use]
    pub fn components(&self) -> &[GasComponent] {
        &self.components
    }

    #[must_use]
    pub fn particles_of(&self, gas: &GasPrototypeId) -> ParticleCount {
        self.components
            .iter()
            .find(|component| &component.gas == gas)
            .map_or(ParticleCount(0), |component| component.particles)
    }

    pub fn set_particles(&mut self, gas: GasPrototypeId, particles: ParticleCount) {
        match self.find_component(&gas) {
            Ok(index) => {
                if particles.0 == 0 {
                    self.components.remove(index);
                } else {
                    self.components[index].particles = particles;
                }
            }
            Err(insert_at) => {
                if particles.0 != 0 {
                    self.components
                        .insert(insert_at, GasComponent { gas, particles });
                }
            }
        }
    }

    pub fn add_particles(
        &mut self,
        gas: GasPrototypeId,
        particles: ParticleCount,
    ) -> Result<(), GasMixtureError> {
        if particles.0 == 0 {
            return Ok(());
        }

        let current = self.particles_of(&gas).0;
        let next =
            current
                .checked_add(particles.0)
                .ok_or_else(|| GasMixtureError::ParticleOverflow {
                    gas: gas.clone(),
                    current,
                    delta: particles.0,
                })?;
        self.set_particles(gas, ParticleCount(next));
        Ok(())
    }

    pub fn remove_particles(
        &mut self,
        gas: GasPrototypeId,
        particles: ParticleCount,
    ) -> Result<(), GasMixtureError> {
        if particles.0 == 0 {
            return Ok(());
        }

        let current = self.particles_of(&gas).0;
        if current < particles.0 {
            return Err(GasMixtureError::NotEnoughParticles {
                gas: gas.clone(),
                available: current,
                requested: particles.0,
            });
        }
        self.set_particles(gas, ParticleCount(current - particles.0));
        Ok(())
    }

    pub fn clear_gas(&mut self, gas: GasPrototypeId) {
        if let Ok(index) = self.find_component(&gas) {
            self.components.remove(index);
        }
    }

    pub fn clear_all(&mut self) {
        self.components.clear();
    }

    #[must_use]
    pub fn total_particles(&self) -> ParticleCount {
        ParticleCount(
            self.components
                .iter()
                .map(|component| component.particles.0)
                .sum(),
        )
    }

    fn find_component(&self, gas: &GasPrototypeId) -> Result<usize, usize> {
        self.components
            .binary_search_by(|component| component.gas.cmp(gas))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GasLayer {
    cells: Vec<GasMixture>,
}

impl GasLayer {
    #[must_use]
    pub fn new(cell_count: usize) -> Self {
        Self {
            cells: vec![GasMixture::default(); cell_count],
        }
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&GasMixture> {
        self.cells.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut GasMixture> {
        self.cells.get_mut(index)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use flux_core::PrototypeId;

    use super::*;

    fn gas_id(value: &str) -> PrototypeId {
        PrototypeId::parse(value).expect("valid gas id")
    }

    #[test]
    fn default_cell_has_no_components() {
        let layer = GasLayer::new(4);
        assert_eq!(layer.len(), 4);
        assert!(
            layer
                .get(0)
                .expect("cell must exist")
                .components()
                .is_empty()
        );
    }

    #[test]
    fn supports_multiple_components_and_total() {
        let oxygen = gas_id("base:gas/oxygen");
        let hydrogen = gas_id("base:gas/hydrogen");
        let mut mixture = GasMixture::default();
        mixture.set_particles(oxygen.clone(), ParticleCount(10));
        mixture.set_particles(hydrogen.clone(), ParticleCount(30));

        assert_eq!(mixture.components().len(), 2);
        assert_eq!(mixture.total_particles(), ParticleCount(40));
        assert_eq!(mixture.particles_of(&oxygen), ParticleCount(10));
        assert_eq!(mixture.particles_of(&hydrogen), ParticleCount(30));
    }

    #[test]
    fn drops_zero_particle_components() {
        let oxygen = gas_id("base:gas/oxygen");
        let mut mixture = GasMixture::default();
        mixture.set_particles(oxygen.clone(), ParticleCount(10));
        mixture.set_particles(oxygen.clone(), ParticleCount(0));

        assert!(mixture.components().is_empty());
    }

    #[test]
    fn duplicate_ids_are_merged_by_sum() {
        let oxygen = gas_id("base:gas/oxygen");
        let mut mixture = GasMixture::default();

        mixture
            .add_particles(oxygen.clone(), ParticleCount(10))
            .expect("first add");
        mixture
            .add_particles(oxygen.clone(), ParticleCount(7))
            .expect("second add");

        assert_eq!(mixture.components().len(), 1);
        assert_eq!(mixture.particles_of(&oxygen), ParticleCount(17));
    }

    #[test]
    fn component_order_is_deterministic() {
        let oxygen = gas_id("base:gas/oxygen");
        let hydrogen = gas_id("base:gas/hydrogen");
        let co2 = gas_id("base:gas/carbon_dioxide");
        let mut mixture = GasMixture::default();

        mixture.set_particles(oxygen.clone(), ParticleCount(1));
        mixture.set_particles(co2.clone(), ParticleCount(1));
        mixture.set_particles(hydrogen.clone(), ParticleCount(1));

        let ids = mixture
            .components()
            .iter()
            .map(|component| component.gas.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
    }

    #[test]
    fn clear_removes_all_components() {
        let oxygen = gas_id("base:gas/oxygen");
        let mut mixture = GasMixture::default();
        mixture.set_particles(oxygen.clone(), ParticleCount(42));
        mixture.clear_gas(oxygen);
        assert!(mixture.components().is_empty());

        let hydrogen = gas_id("base:gas/hydrogen");
        mixture.set_particles(hydrogen, ParticleCount(3));
        mixture.clear_all();
        assert!(mixture.components().is_empty());
    }
}
