use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("flux_app should be nested in workspace")
        .to_path_buf()
}

#[test]
fn run_scenario_reports_missing_id_and_non_zero_exit() {
    let output = Command::new(env!("CARGO_BIN_EXE_flux_app"))
        .args(["--run-scenario", "test_scenarios:scenario/does_not_exist"])
        .current_dir(workspace_root())
        .output()
        .expect("flux_app command should run");

    assert!(
        !output.status.success(),
        "expected non-zero exit code for missing scenario id"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf-8");
    assert!(
        stderr.contains("scenario_id: test_scenarios:scenario/does_not_exist"),
        "stderr should mention missing scenario id, got: {stderr}"
    );
}
