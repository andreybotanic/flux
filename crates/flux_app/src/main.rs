#![forbid(unsafe_code)]

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{InstanceFlags, RenderCreation, WgpuSettings};
use bevy::window::{Window, WindowPlugin};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    Version,
    Windowed,
    Headless,
}

#[derive(Debug, PartialEq, Eq)]
enum CliError {
    UnknownArgument(String),
    ConflictingArguments,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match parse_run_mode(args.iter().map(std::string::String::as_str)) {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };

    match mode {
        RunMode::Version => {
            println!("{}", flux_core::engine_label());
        }
        RunMode::Windowed => run_windowed(),
        RunMode::Headless => run_headless(),
    }
}

fn parse_run_mode<'a, I>(args: I) -> Result<RunMode, CliError>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut wants_version = false;
    let mut wants_headless = false;

    for arg in args {
        match arg {
            "--version" | "-V" => wants_version = true,
            "--headless" => wants_headless = true,
            other => return Err(CliError::UnknownArgument(other.to_owned())),
        }
    }

    if wants_version && wants_headless {
        return Err(CliError::ConflictingArguments);
    }

    if wants_version {
        return Ok(RunMode::Version);
    }

    if wants_headless {
        return Ok(RunMode::Headless);
    }

    Ok(RunMode::Windowed)
}

fn run_windowed() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(LogPlugin {
                filter: "info,wgpu=warn,naga=warn".to_owned(),
                ..Default::default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    instance_flags: InstanceFlags::empty(),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: flux_core::ENGINE_NAME.to_owned(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
    );
    app.add_systems(Startup, windowed_diagnostics);
    app.run();
}

fn run_headless() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LogPlugin {
        filter: "info,wgpu=warn,naga=warn".to_owned(),
        ..Default::default()
    });
    app.add_systems(Startup, headless_diagnostics);
    app.finish();
    app.cleanup();
    app.update();
    info!("headless diagnostics completed");
}

fn windowed_diagnostics() {
    info!("startup mode=windowed engine={}", flux_core::engine_label());
    info!("window initialized");
}

fn headless_diagnostics() {
    info!("startup mode=headless engine={}", flux_core::engine_label());
    info!("headless diagnostics initialized");
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::UnknownArgument(argument) => {
                write!(
                    f,
                    "unknown argument: {argument}. Supported args: --version, -V, --headless"
                )
            }
            CliError::ConflictingArguments => {
                write!(
                    f,
                    "arguments --version and --headless cannot be used together"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_version_flag() {
        assert_eq!(
            parse_run_mode(["--version"].as_slice().iter().copied()),
            Ok(RunMode::Version)
        );
        assert_eq!(
            parse_run_mode(["-V"].as_slice().iter().copied()),
            Ok(RunMode::Version)
        );
    }

    #[test]
    fn parses_headless_flag() {
        assert_eq!(
            parse_run_mode(["--headless"].as_slice().iter().copied()),
            Ok(RunMode::Headless)
        );
    }

    #[test]
    fn defaults_to_windowed_mode() {
        assert_eq!(parse_run_mode(std::iter::empty()), Ok(RunMode::Windowed));
    }

    #[test]
    fn rejects_unknown_argument() {
        assert_eq!(
            parse_run_mode(["--unknown"].as_slice().iter().copied()),
            Err(CliError::UnknownArgument("--unknown".to_owned()))
        );
    }

    #[test]
    fn rejects_conflicting_arguments() {
        assert_eq!(
            parse_run_mode(["--version", "--headless"].as_slice().iter().copied()),
            Err(CliError::ConflictingArguments)
        );
    }
}
