use flux_core::PrototypeId;
use flux_sim::BackendPolicy;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunMode {
    Version,
    ListMods,
    ListContent,
    ListScenarios,
    RunScenario {
        scenario_id: PrototypeId,
        visual_delay_ms: u64,
        backend_policy: BackendPolicy,
    },
    Windowed {
        backend_policy: BackendPolicy,
    },
    Headless,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CliError {
    UnknownArgument(String),
    ConflictingArguments,
    MissingArgumentValue(&'static str),
    InvalidScenarioId(String),
    InvalidScenarioVisualDelay(String),
    ScenarioVisualDelayRequiresRunScenario,
    InvalidBackendPolicy(String),
}

pub(crate) fn parse_run_mode(args: &[String]) -> Result<RunMode, CliError> {
    let mut wants_version = false;
    let mut wants_headless = false;
    let mut wants_list_mods = false;
    let mut wants_list_content = false;
    let mut wants_list_scenarios = false;
    let mut scenario_id: Option<PrototypeId> = None;
    let mut scenario_visual_delay_ms: Option<u64> = None;
    let mut backend_policy = BackendPolicy::CpuOnly;

    let mut index = 0usize;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--version" | "-V" => wants_version = true,
            "--headless" => wants_headless = true,
            "--list-mods" => wants_list_mods = true,
            "--list-content" => wants_list_content = true,
            "--list-scenarios" => wants_list_scenarios = true,
            "--run-scenario" => {
                let value = args
                    .get(index + 1)
                    .ok_or(CliError::MissingArgumentValue("--run-scenario"))?;
                let parsed = PrototypeId::parse(value)
                    .map_err(|_| CliError::InvalidScenarioId(value.clone()))?;
                scenario_id = Some(parsed);
                index += 1;
            }
            "--scenario-visual-delay-ms" => {
                let value = args
                    .get(index + 1)
                    .ok_or(CliError::MissingArgumentValue("--scenario-visual-delay-ms"))?;
                let parsed = value
                    .parse::<u64>()
                    .map_err(|_| CliError::InvalidScenarioVisualDelay(value.clone()))?;
                scenario_visual_delay_ms = Some(parsed);
                index += 1;
            }
            "--backend-policy" => {
                let value = args
                    .get(index + 1)
                    .ok_or(CliError::MissingArgumentValue("--backend-policy"))?;
                backend_policy = BackendPolicy::parse_cli_value(value)
                    .ok_or_else(|| CliError::InvalidBackendPolicy(value.clone()))?;
                index += 1;
            }
            other => return Err(CliError::UnknownArgument(other.to_owned())),
        }
        index += 1;
    }

    let selected_modes = usize::from(wants_version)
        + usize::from(wants_headless)
        + usize::from(wants_list_mods)
        + usize::from(wants_list_content)
        + usize::from(wants_list_scenarios)
        + usize::from(scenario_id.is_some());

    if selected_modes > 1 {
        return Err(CliError::ConflictingArguments);
    }

    if wants_version {
        return Ok(RunMode::Version);
    }

    if wants_list_mods {
        return Ok(RunMode::ListMods);
    }

    if wants_list_content {
        return Ok(RunMode::ListContent);
    }

    if wants_list_scenarios {
        return Ok(RunMode::ListScenarios);
    }

    if scenario_visual_delay_ms.is_some() && scenario_id.is_none() {
        return Err(CliError::ScenarioVisualDelayRequiresRunScenario);
    }

    if let Some(scenario_id) = scenario_id {
        return Ok(RunMode::RunScenario {
            scenario_id,
            visual_delay_ms: scenario_visual_delay_ms.unwrap_or(0),
            backend_policy,
        });
    }

    if wants_headless {
        return Ok(RunMode::Headless);
    }

    Ok(RunMode::Windowed { backend_policy })
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::UnknownArgument(argument) => {
                write!(
                    f,
                    "unknown argument: {argument}. Supported args: --version, -V, --headless, --list-mods, --list-content, --list-scenarios, --run-scenario <id>, --scenario-visual-delay-ms <ms>, --backend-policy <cpu_only|prefer_gpu|prefer_gpu_strict>"
                )
            }
            CliError::ConflictingArguments => {
                write!(
                    f,
                    "arguments --version, --headless, --list-mods, --list-content, --list-scenarios, and --run-scenario are mutually exclusive"
                )
            }
            CliError::MissingArgumentValue(flag) => {
                write!(f, "missing value for argument {flag}")
            }
            CliError::InvalidScenarioId(value) => {
                write!(
                    f,
                    "invalid scenario id `{value}`, expected namespace:path format"
                )
            }
            CliError::InvalidScenarioVisualDelay(value) => write!(
                f,
                "invalid value for --scenario-visual-delay-ms `{value}`, expected non-negative integer milliseconds"
            ),
            CliError::ScenarioVisualDelayRequiresRunScenario => write!(
                f,
                "--scenario-visual-delay-ms can be used only together with --run-scenario <id>"
            ),
            CliError::InvalidBackendPolicy(value) => write!(
                f,
                "invalid value for --backend-policy `{value}`, expected one of: {}",
                BackendPolicy::supported_cli_values().join(", ")
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_version_flag() {
        assert_eq!(
            parse_run_mode(&["--version".to_owned()]),
            Ok(RunMode::Version)
        );
        assert_eq!(parse_run_mode(&["-V".to_owned()]), Ok(RunMode::Version));
    }

    #[test]
    fn parses_headless_flag() {
        assert_eq!(
            parse_run_mode(&["--headless".to_owned()]),
            Ok(RunMode::Headless)
        );
    }

    #[test]
    fn parses_list_mods_flag() {
        assert_eq!(
            parse_run_mode(&["--list-mods".to_owned()]),
            Ok(RunMode::ListMods)
        );
    }

    #[test]
    fn parses_list_content_flag() {
        assert_eq!(
            parse_run_mode(&["--list-content".to_owned()]),
            Ok(RunMode::ListContent)
        );
    }

    #[test]
    fn parses_list_scenarios_flag() {
        assert_eq!(
            parse_run_mode(&["--list-scenarios".to_owned()]),
            Ok(RunMode::ListScenarios)
        );
    }

    #[test]
    fn parses_run_scenario_flag() {
        assert_eq!(
            parse_run_mode(&[
                "--run-scenario".to_owned(),
                "test_scenarios:scenario/bootstrap_smoke".to_owned()
            ]),
            Ok(RunMode::RunScenario {
                scenario_id: PrototypeId::parse("test_scenarios:scenario/bootstrap_smoke")
                    .expect("valid id"),
                visual_delay_ms: 0,
                backend_policy: BackendPolicy::CpuOnly,
            })
        );
    }

    #[test]
    fn parses_run_scenario_with_visual_delay_flag() {
        assert_eq!(
            parse_run_mode(&[
                "--run-scenario".to_owned(),
                "test_scenarios:scenario/bootstrap_smoke".to_owned(),
                "--scenario-visual-delay-ms".to_owned(),
                "250".to_owned(),
            ]),
            Ok(RunMode::RunScenario {
                scenario_id: PrototypeId::parse("test_scenarios:scenario/bootstrap_smoke")
                    .expect("valid id"),
                visual_delay_ms: 250,
                backend_policy: BackendPolicy::CpuOnly,
            })
        );
    }

    #[test]
    fn defaults_to_windowed_mode() {
        assert_eq!(
            parse_run_mode(&[]),
            Ok(RunMode::Windowed {
                backend_policy: BackendPolicy::CpuOnly
            })
        );
    }

    #[test]
    fn parses_backend_policy_flag_for_windowed_mode() {
        assert_eq!(
            parse_run_mode(&["--backend-policy".to_owned(), "prefer_gpu".to_owned()]),
            Ok(RunMode::Windowed {
                backend_policy: BackendPolicy::PreferGpu { cpu_fallback: true }
            })
        );
    }

    #[test]
    fn parses_backend_policy_flag_for_scenario_mode() {
        assert_eq!(
            parse_run_mode(&[
                "--run-scenario".to_owned(),
                "test_scenarios:scenario/bootstrap_smoke".to_owned(),
                "--backend-policy".to_owned(),
                "prefer_gpu_strict".to_owned(),
            ]),
            Ok(RunMode::RunScenario {
                scenario_id: PrototypeId::parse("test_scenarios:scenario/bootstrap_smoke")
                    .expect("valid id"),
                visual_delay_ms: 0,
                backend_policy: BackendPolicy::PreferGpu {
                    cpu_fallback: false
                },
            })
        );
    }

    #[test]
    fn rejects_unknown_argument() {
        assert_eq!(
            parse_run_mode(&["--unknown".to_owned()]),
            Err(CliError::UnknownArgument("--unknown".to_owned()))
        );
    }

    #[test]
    fn rejects_conflicting_arguments() {
        assert_eq!(
            parse_run_mode(&["--version".to_owned(), "--headless".to_owned()]),
            Err(CliError::ConflictingArguments)
        );
        assert_eq!(
            parse_run_mode(&["--list-mods".to_owned(), "--headless".to_owned()]),
            Err(CliError::ConflictingArguments)
        );
        assert_eq!(
            parse_run_mode(&["--version".to_owned(), "--list-mods".to_owned()]),
            Err(CliError::ConflictingArguments)
        );
        assert_eq!(
            parse_run_mode(&["--list-content".to_owned(), "--headless".to_owned()]),
            Err(CliError::ConflictingArguments)
        );
        assert_eq!(
            parse_run_mode(&[
                "--run-scenario".to_owned(),
                "test_scenarios:scenario/bootstrap_smoke".to_owned(),
                "--headless".to_owned()
            ]),
            Err(CliError::ConflictingArguments)
        );
    }

    #[test]
    fn rejects_invalid_run_scenario_arguments() {
        assert_eq!(
            parse_run_mode(&["--run-scenario".to_owned()]),
            Err(CliError::MissingArgumentValue("--run-scenario"))
        );
        assert_eq!(
            parse_run_mode(&["--run-scenario".to_owned(), "invalid".to_owned()]),
            Err(CliError::InvalidScenarioId("invalid".to_owned()))
        );
        assert_eq!(
            parse_run_mode(&["--scenario-visual-delay-ms".to_owned(), "100".to_owned()]),
            Err(CliError::ScenarioVisualDelayRequiresRunScenario)
        );
        assert_eq!(
            parse_run_mode(&[
                "--run-scenario".to_owned(),
                "test_scenarios:scenario/bootstrap_smoke".to_owned(),
                "--scenario-visual-delay-ms".to_owned(),
                "bad".to_owned(),
            ]),
            Err(CliError::InvalidScenarioVisualDelay("bad".to_owned()))
        );
        assert_eq!(
            parse_run_mode(&["--backend-policy".to_owned(), "invalid".to_owned()]),
            Err(CliError::InvalidBackendPolicy("invalid".to_owned()))
        );
    }
}
