use std::collections::BTreeMap;

use flux_core::PrototypeId;

use crate::ContentRegistryError;
use crate::types::{
    GasPrototype, GasRecord, PrototypeKind, PrototypeSource, SolidCellPrototype, SolidCellRecord,
    StructurePrototype, StructureRecord, SubstancePrototype, SubstanceRecord,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryState {
    Building,
    Frozen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegisteredPrototype {
    kind: PrototypeKind,
    source: PrototypeSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContentRegistry {
    state: RegistryState,
    prototype_index: BTreeMap<PrototypeId, RegisteredPrototype>,
    substances: BTreeMap<PrototypeId, SubstanceRecord>,
    solid_cells: BTreeMap<PrototypeId, SolidCellRecord>,
    structures: BTreeMap<PrototypeId, StructureRecord>,
    gases: BTreeMap<PrototypeId, GasRecord>,
}

#[derive(Debug, Clone, Copy)]
pub struct FrozenContentRegistry<'a> {
    registry: &'a ContentRegistry,
}

impl ContentRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: RegistryState::Building,
            prototype_index: BTreeMap::new(),
            substances: BTreeMap::new(),
            solid_cells: BTreeMap::new(),
            structures: BTreeMap::new(),
            gases: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn state(&self) -> RegistryState {
        self.state
    }

    #[must_use]
    pub fn is_frozen(&self) -> bool {
        self.state == RegistryState::Frozen
    }

    pub fn freeze(&mut self) -> FrozenContentRegistry<'_> {
        self.state = RegistryState::Frozen;
        FrozenContentRegistry { registry: self }
    }

    pub fn add_substance(
        &mut self,
        prototype: SubstancePrototype,
        source: PrototypeSource,
    ) -> Result<(), ContentRegistryError> {
        self.ensure_mutable(PrototypeKind::Substance, &prototype.id)?;
        self.ensure_unique(PrototypeKind::Substance, &prototype.id, &source)?;

        let id = prototype.id.clone();
        let source_for_index = source.clone();
        self.substances
            .insert(id.clone(), SubstanceRecord { prototype, source });
        self.prototype_index.insert(
            id,
            RegisteredPrototype {
                kind: PrototypeKind::Substance,
                source: source_for_index,
            },
        );

        Ok(())
    }

    pub fn add_structure(
        &mut self,
        prototype: StructurePrototype,
        source: PrototypeSource,
    ) -> Result<(), ContentRegistryError> {
        self.ensure_mutable(PrototypeKind::Structure, &prototype.id)?;
        self.ensure_unique(PrototypeKind::Structure, &prototype.id, &source)?;

        let id = prototype.id.clone();
        let source_for_index = source.clone();
        self.structures
            .insert(id.clone(), StructureRecord { prototype, source });
        self.prototype_index.insert(
            id,
            RegisteredPrototype {
                kind: PrototypeKind::Structure,
                source: source_for_index,
            },
        );

        Ok(())
    }

    pub fn add_solid_cell(
        &mut self,
        prototype: SolidCellPrototype,
        source: PrototypeSource,
    ) -> Result<(), ContentRegistryError> {
        self.ensure_mutable(PrototypeKind::SolidCell, &prototype.id)?;
        self.ensure_unique(PrototypeKind::SolidCell, &prototype.id, &source)?;

        let id = prototype.id.clone();
        let source_for_index = source.clone();
        self.solid_cells
            .insert(id.clone(), SolidCellRecord { prototype, source });
        self.prototype_index.insert(
            id,
            RegisteredPrototype {
                kind: PrototypeKind::SolidCell,
                source: source_for_index,
            },
        );

        Ok(())
    }

    pub fn add_gas(
        &mut self,
        prototype: GasPrototype,
        source: PrototypeSource,
    ) -> Result<(), ContentRegistryError> {
        self.ensure_mutable(PrototypeKind::Gas, &prototype.id)?;
        self.ensure_unique(PrototypeKind::Gas, &prototype.id, &source)?;

        let id = prototype.id.clone();
        let source_for_index = source.clone();
        self.gases
            .insert(id.clone(), GasRecord { prototype, source });
        self.prototype_index.insert(
            id,
            RegisteredPrototype {
                kind: PrototypeKind::Gas,
                source: source_for_index,
            },
        );

        Ok(())
    }

    pub fn substances(&self) -> impl Iterator<Item = &SubstanceRecord> {
        self.substances.values()
    }

    pub fn solid_cells(&self) -> impl Iterator<Item = &SolidCellRecord> {
        self.solid_cells.values()
    }

    pub fn structures(&self) -> impl Iterator<Item = &StructureRecord> {
        self.structures.values()
    }

    pub fn gases(&self) -> impl Iterator<Item = &GasRecord> {
        self.gases.values()
    }

    #[must_use]
    pub fn substances_len(&self) -> usize {
        self.substances.len()
    }

    #[must_use]
    pub fn solid_cells_len(&self) -> usize {
        self.solid_cells.len()
    }

    #[must_use]
    pub fn structures_len(&self) -> usize {
        self.structures.len()
    }

    #[must_use]
    pub fn gases_len(&self) -> usize {
        self.gases.len()
    }

    fn ensure_mutable(
        &self,
        prototype_kind: PrototypeKind,
        prototype_id: &PrototypeId,
    ) -> Result<(), ContentRegistryError> {
        if self.is_frozen() {
            return Err(ContentRegistryError::RegistryFrozenMutation {
                prototype_kind: prototype_kind.as_str().into(),
                prototype_id: prototype_id.to_string().into(),
            });
        }

        Ok(())
    }

    fn ensure_unique(
        &self,
        duplicate_kind: PrototypeKind,
        prototype_id: &PrototypeId,
        duplicate_source: &PrototypeSource,
    ) -> Result<(), ContentRegistryError> {
        if let Some(existing) = self.prototype_index.get(prototype_id) {
            return Err(ContentRegistryError::DuplicatePrototypeId {
                prototype_id: prototype_id.to_string().into(),
                existing_kind: existing.kind.as_str().into(),
                existing_mod: existing.source.mod_id.clone().into(),
                existing_file: existing.source.file.clone().into(),
                duplicate_kind: duplicate_kind.as_str().into(),
                duplicate_mod: duplicate_source.mod_id.clone().into(),
                duplicate_file: duplicate_source.file.clone().into(),
            });
        }

        Ok(())
    }
}

impl Default for ContentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> FrozenContentRegistry<'a> {
    #[must_use]
    pub fn registry(&self) -> &'a ContentRegistry {
        self.registry
    }
}

#[cfg(test)]
mod tests {
    use flux_core::PrototypeId;

    use super::*;
    use crate::types::{GasPrototype, LocalizationKey, TileSize};

    #[test]
    fn rejects_mutation_after_freeze() {
        let mut registry = ContentRegistry::new();
        registry.freeze();

        let result = registry.add_substance(
            SubstancePrototype {
                id: PrototypeId::parse("base:material/oxygen").expect("valid id"),
                display_name: LocalizationKey::parse("base.substance.oxygen").expect("valid key"),
            },
            PrototypeSource {
                mod_id: "base".to_owned(),
                file: "mods/base/content/substances/oxygen.ron".to_owned(),
            },
        );

        assert!(matches!(
            result,
            Err(ContentRegistryError::RegistryFrozenMutation { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_id_across_kinds() {
        let mut registry = ContentRegistry::new();
        let id = PrototypeId::parse("base:material/oxygen").expect("valid id");

        registry
            .add_substance(
                SubstancePrototype {
                    id: id.clone(),
                    display_name: LocalizationKey::parse("base.substance.oxygen")
                        .expect("valid key"),
                },
                PrototypeSource {
                    mod_id: "base".to_owned(),
                    file: "mods/base/content/substances/oxygen.ron".to_owned(),
                },
            )
            .expect("must add");

        let duplicate = registry.add_structure(
            StructurePrototype {
                id,
                display_name: LocalizationKey::parse("base.structure.oxygen").expect("valid key"),
                size: TileSize {
                    width: 1,
                    height: 1,
                },
            },
            PrototypeSource {
                mod_id: "base".to_owned(),
                file: "mods/base/content/structures/oxygen.ron".to_owned(),
            },
        );

        assert!(matches!(
            duplicate,
            Err(ContentRegistryError::DuplicatePrototypeId { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_id_between_substance_and_gas() {
        let mut registry = ContentRegistry::new();
        let id = PrototypeId::parse("base:material/oxygen").expect("valid id");

        registry
            .add_substance(
                SubstancePrototype {
                    id: id.clone(),
                    display_name: LocalizationKey::parse("base.substance.oxygen")
                        .expect("valid key"),
                },
                PrototypeSource {
                    mod_id: "base".to_owned(),
                    file: "mods/base/content/substances/oxygen.ron".to_owned(),
                },
            )
            .expect("must add");

        let duplicate = registry.add_gas(
            GasPrototype {
                id,
                display_name: LocalizationKey::parse("base.gas.oxygen").expect("valid key"),
                molar_mass: 31.998,
            },
            PrototypeSource {
                mod_id: "base".to_owned(),
                file: "mods/base/content/gases/oxygen.ron".to_owned(),
            },
        );

        assert!(matches!(
            duplicate,
            Err(ContentRegistryError::DuplicatePrototypeId { .. })
        ));
    }
}
