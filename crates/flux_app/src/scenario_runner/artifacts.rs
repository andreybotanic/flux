use std::path::{Path, PathBuf};

use flux_core::PrototypeId;

pub(super) fn scenario_artifact_dir(scenario_id: &PrototypeId) -> PathBuf {
    Path::new("logs")
        .join("scenarios")
        .join(scenario_id.namespace())
        .join(scenario_id.path())
}
