#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::Path;

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{InstanceFlags, RenderCreation, WgpuSettings};
use bevy::window::{Window, WindowPlugin};
use flux_mod_loader::DiscoveredMod;
use flux_world::{GridSize, WorldGrid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    Version,
    ListMods,
    ListContent,
    WorldDebugCreate { size: GridSize, chunk_size: u32 },
    Windowed,
    Headless,
}

#[derive(Debug, PartialEq, Eq)]
enum CliError {
    UnknownArgument(String),
    ConflictingArguments,
    MissingArgumentValue(&'static str),
    InvalidWorldSize(String),
    InvalidChunkSize(String),
    ChunkSizeWithoutWorldDebug,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match parse_run_mode(&args) {
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
        RunMode::ListMods => {
            let exit_code = run_list_mods();
            std::process::exit(exit_code);
        }
        RunMode::ListContent => {
            let exit_code = run_list_content();
            std::process::exit(exit_code);
        }
        RunMode::WorldDebugCreate { size, chunk_size } => {
            let exit_code = run_world_debug_create(size, chunk_size);
            std::process::exit(exit_code);
        }
        RunMode::Windowed => run_windowed(),
        RunMode::Headless => run_headless(),
    }
}

fn parse_run_mode(args: &[String]) -> Result<RunMode, CliError> {
    let mut wants_version = false;
    let mut wants_headless = false;
    let mut wants_list_mods = false;
    let mut wants_list_content = false;
    let mut world_size: Option<GridSize> = None;
    let mut chunk_size: Option<u32> = None;

    let mut index = 0usize;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--version" | "-V" => wants_version = true,
            "--headless" => wants_headless = true,
            "--list-mods" => wants_list_mods = true,
            "--list-content" => wants_list_content = true,
            "--world-debug-create" => {
                let value = args
                    .get(index + 1)
                    .ok_or(CliError::MissingArgumentValue("--world-debug-create"))?;
                world_size = Some(parse_world_size(value)?);
                index += 1;
            }
            "--chunk-size" => {
                let value = args
                    .get(index + 1)
                    .ok_or(CliError::MissingArgumentValue("--chunk-size"))?;
                let parsed = value
                    .parse::<u32>()
                    .map_err(|_| CliError::InvalidChunkSize(value.clone()))?;
                if parsed == 0 {
                    return Err(CliError::InvalidChunkSize(value.clone()));
                }
                chunk_size = Some(parsed);
                index += 1;
            }
            other => return Err(CliError::UnknownArgument(other.to_owned())),
        }
        index += 1;
    }

    if chunk_size.is_some() && world_size.is_none() {
        return Err(CliError::ChunkSizeWithoutWorldDebug);
    }

    let selected_modes = usize::from(wants_version)
        + usize::from(wants_headless)
        + usize::from(wants_list_mods)
        + usize::from(wants_list_content)
        + usize::from(world_size.is_some());

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

    if let Some(size) = world_size {
        return Ok(RunMode::WorldDebugCreate {
            size,
            chunk_size: chunk_size.unwrap_or(16),
        });
    }

    if wants_headless {
        return Ok(RunMode::Headless);
    }

    Ok(RunMode::Windowed)
}

fn parse_world_size(value: &str) -> Result<GridSize, CliError> {
    let Some((width_raw, height_raw)) = value.split_once('x') else {
        return Err(CliError::InvalidWorldSize(value.to_owned()));
    };
    let width = width_raw
        .parse::<u32>()
        .map_err(|_| CliError::InvalidWorldSize(value.to_owned()))?;
    let height = height_raw
        .parse::<u32>()
        .map_err(|_| CliError::InvalidWorldSize(value.to_owned()))?;
    if width == 0 || height == 0 {
        return Err(CliError::InvalidWorldSize(value.to_owned()));
    }
    Ok(GridSize::new(width, height))
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

fn run_list_mods() -> i32 {
    let report = flux_mod_loader::discover_and_resolve_mods(Path::new("mods"));
    let by_mod_id: BTreeMap<&str, &DiscoveredMod> = report
        .valid_mods
        .iter()
        .map(|module| (module.manifest.mod_id.as_str(), module))
        .collect();

    println!("valid mods: {}", report.valid_mods.len());
    if !report.valid_mods.is_empty() {
        if report.errors.is_empty() {
            let order = report
                .resolved_order
                .as_ref()
                .expect("resolved order must exist when there are no errors");
            println!("resolved load order:");
            for mod_id in &order.ordered_mod_ids {
                if let Some(module) = by_mod_id.get(mod_id.as_str()) {
                    print_discovered_mod(module);
                }
            }
        } else {
            println!("valid mods (unordered due to errors):");
            for module in &report.valid_mods {
                print_discovered_mod(module);
            }
        }
    }

    if !report.errors.is_empty() {
        eprintln!("errors: {}", report.errors.len());
        for error in &report.errors {
            eprintln!("{error}");
        }
        return 1;
    }

    0
}

fn run_list_content() -> i32 {
    let report = flux_mod_loader::discover_and_resolve_mods(Path::new("mods"));

    if !report.errors.is_empty() {
        eprintln!("errors: {}", report.errors.len());
        for error in &report.errors {
            eprintln!("{error}");
        }
        return 1;
    }

    let resolved_order = report
        .resolved_order
        .as_ref()
        .expect("resolved order must exist when there are no mod errors");

    let content_report = flux_content::load_content_registry(&report.valid_mods, resolved_order);
    if !content_report.errors.is_empty() {
        eprintln!("errors: {}", content_report.errors.len());
        for error in &content_report.errors {
            eprintln!("{error}");
        }
        return 1;
    }

    let registry = content_report
        .registry
        .expect("content registry must exist when there are no content errors");

    println!(
        "content summary: solid_cells={} substances={} structures={} gases={}",
        registry.solid_cells_len(),
        registry.substances_len(),
        registry.structures_len(),
        registry.gases_len()
    );

    println!("solid cells:");
    for record in registry.solid_cells() {
        println!(
            "- id={} display_name={} gas_permeable={} source_mod={} source_file={}",
            record.prototype.id,
            record.prototype.display_name,
            record.prototype.gas_permeable,
            record.source.mod_id,
            record.source.file
        );
        print_patch_trail(&registry, &record.prototype.id);
    }

    println!("substances:");
    for record in registry.substances() {
        println!(
            "- id={} display_name={} source_mod={} source_file={}",
            record.prototype.id,
            record.prototype.display_name,
            record.source.mod_id,
            record.source.file
        );
        print_patch_trail(&registry, &record.prototype.id);
    }

    println!("structures:");
    for record in registry.structures() {
        println!(
            "- id={} display_name={} size={}x{} source_mod={} source_file={}",
            record.prototype.id,
            record.prototype.display_name,
            record.prototype.size.width,
            record.prototype.size.height,
            record.source.mod_id,
            record.source.file
        );
        print_patch_trail(&registry, &record.prototype.id);
    }

    println!("gases:");
    for record in registry.gases() {
        println!(
            "- id={} display_name={} molar_mass={} source_mod={} source_file={}",
            record.prototype.id,
            record.prototype.display_name,
            record.prototype.molar_mass,
            record.source.mod_id,
            record.source.file
        );
        print_patch_trail(&registry, &record.prototype.id);
    }

    0
}

fn run_world_debug_create(size: GridSize, chunk_size: u32) -> i32 {
    let mut world = match WorldGrid::new(size, chunk_size) {
        Ok(world) => world,
        Err(error) => {
            eprintln!("failed to create world: {error}");
            return 1;
        }
    };

    let report = flux_mod_loader::discover_and_resolve_mods(Path::new("mods"));
    if report.errors.is_empty() {
        if let Some(resolved_order) = report.resolved_order.as_ref() {
            let content_report =
                flux_content::load_content_registry(&report.valid_mods, resolved_order);
            if content_report.errors.is_empty() {
                if let Some(registry) = content_report.registry.as_ref() {
                    let count = world.refresh_structure_sizes_from_registry(registry);
                    println!("structure prototype sizes loaded: {count}");
                }
            } else {
                eprintln!(
                    "warning: content registry has {} error(s), continuing with empty structure size lookup",
                    content_report.errors.len()
                );
            }
        }
    } else {
        eprintln!(
            "warning: mod discovery has {} error(s), continuing with empty structure size lookup",
            report.errors.len()
        );
    }

    println!(
        "world summary: size={}x{} cells={} chunk_size={} chunks={} chunk_grid={}x{}",
        world.size().width,
        world.size().height,
        world.cell_count(),
        world.chunk_size(),
        world.chunks().len(),
        world.chunk_cols(),
        world.chunk_rows()
    );
    0
}

fn print_patch_trail(
    registry: &flux_content::ContentRegistry,
    prototype_id: &flux_core::PrototypeId,
) {
    let patches = registry
        .applied_patches_for(prototype_id)
        .collect::<Vec<_>>();
    if patches.is_empty() {
        return;
    }

    println!("  patches:");
    for patch in patches {
        println!(
            "  - kind={} source_mod={} source_file={}",
            patch.patch_kind.as_str(),
            patch.source.mod_id,
            patch.source.file
        );
    }
}

fn print_discovered_mod(module: &DiscoveredMod) {
    println!(
        "- id={} version={} api_version={} path={}",
        module.manifest.mod_id,
        module.manifest.version,
        module.manifest.api_version,
        module.directory_path.display()
    );
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::UnknownArgument(argument) => {
                write!(
                    f,
                    "unknown argument: {argument}. Supported args: --version, -V, --headless, --list-mods, --list-content, --world-debug-create <WxH>, --chunk-size <N>"
                )
            }
            CliError::ConflictingArguments => {
                write!(
                    f,
                    "arguments --version, --headless, --list-mods, --list-content, and --world-debug-create are mutually exclusive"
                )
            }
            CliError::MissingArgumentValue(flag) => {
                write!(f, "missing value for argument {flag}")
            }
            CliError::InvalidWorldSize(value) => {
                write!(
                    f,
                    "invalid world size `{value}`, expected format WxH with positive integers"
                )
            }
            CliError::InvalidChunkSize(value) => {
                write!(f, "invalid chunk size `{value}`, expected positive integer")
            }
            CliError::ChunkSizeWithoutWorldDebug => {
                write!(
                    f,
                    "argument --chunk-size can only be used with --world-debug-create"
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
    fn parses_world_debug_create_flag() {
        assert_eq!(
            parse_run_mode(&["--world-debug-create".to_owned(), "64x64".to_owned()]),
            Ok(RunMode::WorldDebugCreate {
                size: GridSize::new(64, 64),
                chunk_size: 16
            })
        );
        assert_eq!(
            parse_run_mode(&[
                "--world-debug-create".to_owned(),
                "64x64".to_owned(),
                "--chunk-size".to_owned(),
                "8".to_owned()
            ]),
            Ok(RunMode::WorldDebugCreate {
                size: GridSize::new(64, 64),
                chunk_size: 8
            })
        );
    }

    #[test]
    fn defaults_to_windowed_mode() {
        assert_eq!(parse_run_mode(&[]), Ok(RunMode::Windowed));
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
                "--world-debug-create".to_owned(),
                "32x32".to_owned(),
                "--headless".to_owned()
            ]),
            Err(CliError::ConflictingArguments)
        );
    }

    #[test]
    fn rejects_invalid_world_debug_arguments() {
        assert_eq!(
            parse_run_mode(&["--world-debug-create".to_owned()]),
            Err(CliError::MissingArgumentValue("--world-debug-create"))
        );
        assert_eq!(
            parse_run_mode(&["--world-debug-create".to_owned(), "64".to_owned()]),
            Err(CliError::InvalidWorldSize("64".to_owned()))
        );
        assert_eq!(
            parse_run_mode(&["--chunk-size".to_owned(), "16".to_owned()]),
            Err(CliError::ChunkSizeWithoutWorldDebug)
        );
    }
}
