use std::collections::BTreeMap;

use flux_core::PrototypeId;

use crate::ContentRegistryError;
use crate::types::{
    AppliedPrototypePatch, GasPrototype, GasRecord, PrototypeBody, PrototypeKind, PrototypePatch,
    PrototypePatchBody, PrototypePatchFor, PrototypeSource, PrototypeValidate, SolidCellPrototype,
    SolidCellRecord, StructurePrototype, StructureRecord, SubstancePrototype, SubstanceRecord,
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
    applied_patches: BTreeMap<PrototypeId, Vec<AppliedPrototypePatch>>,
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
            applied_patches: BTreeMap::new(),
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

    pub fn add_prototype(
        &mut self,
        prototype: PrototypeBody,
        source: PrototypeSource,
    ) -> Result<(), ContentRegistryError> {
        match prototype {
            PrototypeBody::SubstancePrototype(prototype) => self.add_substance(prototype, source),
            PrototypeBody::SolidCellPrototype(prototype) => self.add_solid_cell(prototype, source),
            PrototypeBody::StructurePrototype(prototype) => self.add_structure(prototype, source),
            PrototypeBody::GasPrototype(prototype) => self.add_gas(prototype, source),
        }
    }

    pub fn apply_patch(
        &mut self,
        patch: PrototypePatch,
        source: PrototypeSource,
    ) -> Result<(), ContentRegistryError> {
        let patch_kind = patch.body.kind();
        self.ensure_mutable(patch_kind, &patch.target)?;

        if patch.body.is_empty() {
            return Err(ContentRegistryError::EmptyPatchBody {
                mod_id: source.mod_id.clone().into(),
                file: source.file.clone().into(),
                target: patch.target.to_string().into(),
                patch_kind: patch_kind.as_str().into(),
            });
        }

        let target_kind = self
            .prototype_index
            .get(&patch.target)
            .ok_or_else(|| ContentRegistryError::PatchTargetNotFound {
                mod_id: source.mod_id.clone().into(),
                file: source.file.clone().into(),
                target: patch.target.to_string().into(),
            })?
            .kind;

        if target_kind != patch_kind {
            return Err(ContentRegistryError::PatchKindMismatch {
                mod_id: source.mod_id.clone().into(),
                file: source.file.clone().into(),
                target: patch.target.to_string().into(),
                target_kind: target_kind.as_str().into(),
                patch_kind: patch_kind.as_str().into(),
            });
        }

        match patch.body {
            PrototypePatchBody::Substance(body) => {
                let record = self
                    .substances
                    .get_mut(&patch.target)
                    .expect("index in sync");
                Self::apply_patch_body(&mut record.prototype, body, &source, &patch.target)?;
            }
            PrototypePatchBody::SolidCell(body) => {
                let record = self
                    .solid_cells
                    .get_mut(&patch.target)
                    .expect("index in sync");
                Self::apply_patch_body(&mut record.prototype, body, &source, &patch.target)?;
            }
            PrototypePatchBody::Structure(body) => {
                let record = self
                    .structures
                    .get_mut(&patch.target)
                    .expect("index in sync");
                Self::apply_patch_body(&mut record.prototype, body, &source, &patch.target)?;
            }
            PrototypePatchBody::Gas(body) => {
                let record = self.gases.get_mut(&patch.target).expect("index in sync");
                Self::apply_patch_body(&mut record.prototype, body, &source, &patch.target)?;
            }
        }

        self.applied_patches
            .entry(patch.target)
            .or_default()
            .push(AppliedPrototypePatch { source, patch_kind });

        Ok(())
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

    #[must_use]
    pub fn prototype_kind(&self, prototype_id: &PrototypeId) -> Option<PrototypeKind> {
        self.prototype_index
            .get(prototype_id)
            .map(|entry| entry.kind)
    }

    pub fn applied_patches_for(
        &self,
        prototype_id: &PrototypeId,
    ) -> impl Iterator<Item = &AppliedPrototypePatch> {
        self.applied_patches
            .get(prototype_id)
            .into_iter()
            .flat_map(|patches| patches.iter())
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

    fn apply_patch_body<P>(
        prototype: &mut P,
        body: P::Patch,
        source: &PrototypeSource,
        target: &PrototypeId,
    ) -> Result<(), ContentRegistryError>
    where
        P: crate::types::Prototype + PrototypeValidate,
    {
        body.apply_to(prototype)
            .map_err(|reason| ContentRegistryError::PatchApplyFailed {
                mod_id: source.mod_id.clone().into(),
                file: source.file.clone().into(),
                target: target.to_string().into(),
                reason: reason.into(),
            })?;
        prototype.validate(source)
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
