use std::collections::{BTreeMap, BTreeSet};

use flux_core::ModId;

use crate::types::{DiscoveredMod, ModLoaderError, ResolvedModOrder};

pub(crate) fn resolve_load_order(
    valid_mods: &[DiscoveredMod],
) -> Result<ResolvedModOrder, ModLoaderError> {
    if valid_mods.is_empty() {
        return Ok(ResolvedModOrder {
            ordered_mod_ids: Vec::new(),
        });
    }

    let mut in_degree: BTreeMap<ModId, usize> = BTreeMap::new();
    let mut edges: BTreeMap<ModId, Vec<ModId>> = BTreeMap::new();

    for module in valid_mods {
        let mod_id = module.manifest.mod_id.clone();
        in_degree.entry(mod_id.clone()).or_insert(0);
        edges.entry(mod_id.clone()).or_default();

        for dependency_id in module.manifest.dependencies.keys() {
            let dependents = edges.entry(dependency_id.clone()).or_default();
            dependents.push(mod_id.clone());
            *in_degree.entry(mod_id.clone()).or_insert(0) += 1;
        }
    }

    let mut ready = BTreeSet::new();
    for (mod_id, degree) in &in_degree {
        if *degree == 0 {
            ready.insert(mod_id.clone());
        }
    }

    let mut ordered_mod_ids = Vec::new();
    while let Some(next) = ready.iter().next().cloned() {
        ready.remove(&next);
        ordered_mod_ids.push(next.clone());

        let dependents = edges.get(&next).cloned().unwrap_or_default();
        for dependent in dependents {
            if let Some(current_degree) = in_degree.get_mut(&dependent) {
                *current_degree = current_degree.saturating_sub(1);
                if *current_degree == 0 {
                    ready.insert(dependent);
                }
            }
        }
    }

    if ordered_mod_ids.len() != valid_mods.len() {
        let cycle_nodes = detect_dependency_cycle(valid_mods)
            .map(|cycle| {
                cycle
                    .iter()
                    .map(ModId::as_str)
                    .collect::<Vec<_>>()
                    .join(" -> ")
            })
            .unwrap_or_else(|| {
                in_degree
                    .iter()
                    .filter_map(|(mod_id, degree)| (*degree > 0).then_some(mod_id.as_str()))
                    .collect::<Vec<_>>()
                    .join(" -> ")
            });
        return Err(ModLoaderError::DependencyCycle { cycle: cycle_nodes });
    }

    Ok(ResolvedModOrder { ordered_mod_ids })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

fn detect_dependency_cycle(valid_mods: &[DiscoveredMod]) -> Option<Vec<ModId>> {
    let known_ids: BTreeSet<ModId> = valid_mods
        .iter()
        .map(|module| module.manifest.mod_id.clone())
        .collect();
    let mut dependencies_by_mod: BTreeMap<ModId, Vec<ModId>> = BTreeMap::new();

    for module in valid_mods {
        let mut dependencies = module
            .manifest
            .dependencies
            .keys()
            .filter(|dependency_id| known_ids.contains(*dependency_id))
            .cloned()
            .collect::<Vec<_>>();
        dependencies.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        dependencies_by_mod.insert(module.manifest.mod_id.clone(), dependencies);
    }

    let mut states: BTreeMap<ModId, VisitState> = dependencies_by_mod
        .keys()
        .cloned()
        .map(|mod_id| (mod_id, VisitState::Unvisited))
        .collect();
    let mut stack: Vec<ModId> = Vec::new();
    let mut stack_indices: BTreeMap<ModId, usize> = BTreeMap::new();

    for mod_id in dependencies_by_mod.keys() {
        if states.get(mod_id) != Some(&VisitState::Unvisited) {
            continue;
        }

        if let Some(cycle) = dfs_find_cycle(
            mod_id,
            &dependencies_by_mod,
            &mut states,
            &mut stack,
            &mut stack_indices,
        ) {
            return Some(cycle);
        }
    }

    None
}

fn dfs_find_cycle(
    mod_id: &ModId,
    dependencies_by_mod: &BTreeMap<ModId, Vec<ModId>>,
    states: &mut BTreeMap<ModId, VisitState>,
    stack: &mut Vec<ModId>,
    stack_indices: &mut BTreeMap<ModId, usize>,
) -> Option<Vec<ModId>> {
    states.insert(mod_id.clone(), VisitState::Visiting);
    stack_indices.insert(mod_id.clone(), stack.len());
    stack.push(mod_id.clone());

    let neighbors = dependencies_by_mod
        .get(mod_id)
        .expect("dependencies map must contain every mod");
    for dependency_id in neighbors {
        match states
            .get(dependency_id)
            .copied()
            .unwrap_or(VisitState::Unvisited)
        {
            VisitState::Unvisited => {
                if let Some(cycle) = dfs_find_cycle(
                    dependency_id,
                    dependencies_by_mod,
                    states,
                    stack,
                    stack_indices,
                ) {
                    return Some(cycle);
                }
            }
            VisitState::Visiting => {
                if let Some(start) = stack_indices.get(dependency_id).copied() {
                    let mut cycle = stack[start..].to_vec();
                    cycle.push(dependency_id.clone());
                    return Some(cycle);
                }
            }
            VisitState::Visited => {}
        }
    }

    stack.pop();
    stack_indices.remove(mod_id);
    states.insert(mod_id.clone(), VisitState::Visited);
    None
}
