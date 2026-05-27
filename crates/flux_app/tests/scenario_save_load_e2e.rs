use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use flux_content::{
    AssetPath, LocalizationKey, PrototypeSource, SingleSpriteVisual, StructurePrototype, TileSize,
    VisualDefinition, VisualDefinitionKind,
};
use flux_save::{load_game, save_game};
use flux_world::{GridSize, ParticleCount, TilePos, WorldGrid};

static SAVE_TEST_MUTEX: Mutex<()> = Mutex::new(());
const SCENARIO_GAS_SLOT_A: &str = "scenario_gas_slot_a";
const SCENARIO_GAS_SLOT_B: &str = "scenario_gas_slot_b";

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("flux_app should be nested in workspace")
        .to_path_buf()
}

fn scenario_artifact_dir(scenario_id: &str) -> PathBuf {
    let (namespace, path) = scenario_id
        .split_once(':')
        .expect("scenario id must be namespace:path");
    workspace_root()
        .join("logs")
        .join("scenarios")
        .join(namespace)
        .join(path)
}

fn normalize_runtime_log_lines(raw: &str) -> Vec<String> {
    raw.lines()
        .map(|line| {
            line.split_once("flux_app::scenario_runner::runtime: ")
                .map_or_else(
                    || line.trim().to_owned(),
                    |(_, tail)| tail.trim().to_owned(),
                )
        })
        .collect()
}

fn assert_substrings_in_order(lines: &[String], expected_substrings: &[&str]) {
    let mut cursor = 0usize;
    for expected in expected_substrings {
        let mut found = false;
        while cursor < lines.len() {
            if lines[cursor].contains(expected) {
                found = true;
                cursor += 1;
                break;
            }
            cursor += 1;
        }
        assert!(
            found,
            "expected log fragment not found in order: `{expected}`"
        );
    }
}

fn run_flux_app(args: &[&str]) {
    let output = Command::new(env!("CARGO_BIN_EXE_flux_app"))
        .args(args)
        .current_dir(workspace_root())
        .output()
        .expect("flux_app command should run");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "command failed: flux_app {}\nstatus={}\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            output.status,
            stdout,
            stderr
        );
    }
}

fn read_log_lines(scenario_id: &str) -> Vec<String> {
    let log_path = scenario_artifact_dir(scenario_id).join("diagnostic.log");
    let raw = fs::read_to_string(&log_path)
        .unwrap_or_else(|error| panic!("failed to read `{}`: {error}", log_path.display()));
    normalize_runtime_log_lines(&raw)
}

fn open_png(path: &Path) -> image::RgbaImage {
    let dynamic = image::open(path)
        .unwrap_or_else(|error| panic!("failed to decode PNG `{}`: {error}", path.display()));
    dynamic.to_rgba8()
}

fn count_content_pixels(image: &image::RgbaImage) -> usize {
    image
        .pixels()
        .filter(|pixel| {
            let [r, g, b, a] = pixel.0;
            if a <= 200 {
                return false;
            }
            let is_background =
                (19..=27).contains(&r) && (22..=31).contains(&g) && (27..=36).contains(&b);
            let is_grid_line =
                (64..=75).contains(&r) && (69..=82).contains(&g) && (76..=92).contains(&b);
            !(is_background || is_grid_line)
        })
        .count()
}

fn pixel_difference_ratio(left: &image::RgbaImage, right: &image::RgbaImage) -> f32 {
    assert_eq!(left.dimensions(), right.dimensions());
    let mut different = 0usize;
    let total = (left.width() as usize) * (left.height() as usize);
    for (a, b) in left.pixels().zip(right.pixels()) {
        let delta = a.0[0].abs_diff(b.0[0]) as u16
            + a.0[1].abs_diff(b.0[1]) as u16
            + a.0[2].abs_diff(b.0[2]) as u16
            + a.0[3].abs_diff(b.0[3]) as u16;
        if delta > 20 {
            different += 1;
        }
    }
    different as f32 / total as f32
}

fn count_blue_menu_pixels(image: &image::RgbaImage) -> usize {
    image
        .pixels()
        .filter(|pixel| {
            let [r, g, b, a] = pixel.0;
            a > 200 && b > 140 && b > r.saturating_add(20) && b > g.saturating_add(10)
        })
        .count()
}

fn load_registry_for_save_layer_decode() -> flux_content::ContentRegistry {
    fn id(value: &str) -> flux_core::PrototypeId {
        flux_core::PrototypeId::parse(value).expect("valid id")
    }
    fn key(value: &str) -> LocalizationKey {
        LocalizationKey::parse(value).expect("valid key")
    }
    fn visual(path: &str) -> VisualDefinition {
        VisualDefinition {
            kind: VisualDefinitionKind::SingleSprite(SingleSpriteVisual {
                image: AssetPath::parse(path).expect("valid asset path"),
            }),
        }
    }

    let mut registry = flux_content::ContentRegistry::new();
    registry
        .add_structure(
            StructurePrototype {
                id: id("base:building/gas_pump"),
                display_name: key("base.structure.gas_pump"),
                size: TileSize {
                    width: 2,
                    height: 1,
                },
                visual: visual("textures/structure/gas_pump.png"),
            },
            PrototypeSource {
                mod_id: "base".to_owned(),
                file: "mods/base/content/structures/gas_pump.ron".to_owned(),
            },
        )
        .expect("gas_pump must be accepted");
    registry
        .add_structure(
            StructurePrototype {
                id: id("base:building/vent"),
                display_name: key("base.structure.vent"),
                size: TileSize {
                    width: 1,
                    height: 1,
                },
                visual: visual("textures/structure/vent.png"),
            },
            PrototypeSource {
                mod_id: "base".to_owned(),
                file: "mods/base/content/structures/vent.ron".to_owned(),
            },
        )
        .expect("vent must be accepted");
    registry.freeze();
    registry
}

fn generate_slot_saves(saves_dir: &Path, registry: &flux_content::ContentRegistry) {
    fn id(value: &str) -> flux_core::PrototypeId {
        flux_core::PrototypeId::parse(value).expect("valid id")
    }

    fn fill_solid_rect(
        world: &mut WorldGrid,
        solid_id: flux_core::PrototypeId,
        origin_x: u32,
        origin_y: u32,
        width: u32,
        height: u32,
    ) {
        for y in origin_y..origin_y + height {
            for x in origin_x..origin_x + width {
                world
                    .set_solid_cell_at(TilePos::new(x, y), Some(solid_id.clone()))
                    .expect("solid fill");
            }
        }
    }

    fn fill_gas_rect(
        world: &mut WorldGrid,
        gas_id: flux_core::PrototypeId,
        origin_x: u32,
        origin_y: u32,
        width: u32,
        height: u32,
        particles: u64,
    ) {
        for y in origin_y..origin_y + height {
            for x in origin_x..origin_x + width {
                world
                    .set_gas_particles(TilePos::new(x, y), gas_id.clone(), ParticleCount(particles))
                    .expect("gas fill");
            }
        }
    }

    fn place_structure(
        world: &mut WorldGrid,
        structure_id: flux_core::PrototypeId,
        x: u32,
        y: u32,
    ) {
        world
            .place_structure(structure_id, TilePos::new(x, y))
            .expect("structure placement");
    }

    let floor_cell = id("base:solid_cell/floor_cell");
    let oxygen = id("base:gas/oxygen");
    let hydrogen = id("base:gas/hydrogen");
    let gas_pump = id("base:building/gas_pump");
    let vent = id("base:building/vent");

    let mut slot_a_world = WorldGrid::new(GridSize::new(30, 18)).expect("slot_a world");
    slot_a_world.refresh_structure_sizes_from_registry(registry);
    fill_solid_rect(&mut slot_a_world, floor_cell.clone(), 3, 11, 22, 4);
    fill_gas_rect(&mut slot_a_world, oxygen, 4, 3, 7, 5, 260);
    place_structure(&mut slot_a_world, gas_pump.clone(), 7, 10);
    place_structure(&mut slot_a_world, vent.clone(), 22, 10);
    save_game(saves_dir, "slot_a", &slot_a_world, 11, 3).expect("save slot_a");

    let mut slot_b_world = WorldGrid::new(GridSize::new(30, 18)).expect("slot_b world");
    slot_b_world.refresh_structure_sizes_from_registry(registry);
    fill_solid_rect(&mut slot_b_world, floor_cell, 12, 2, 5, 14);
    fill_gas_rect(&mut slot_b_world, hydrogen, 17, 5, 9, 7, 480);
    place_structure(&mut slot_b_world, gas_pump, 20, 6);
    place_structure(&mut slot_b_world, vent, 10, 12);
    save_game(saves_dir, "slot_b", &slot_b_world, 99, 4).expect("save slot_b");
}

fn generate_diffusion_slot_saves(saves_dir: &Path) {
    fn id(value: &str) -> flux_core::PrototypeId {
        flux_core::PrototypeId::parse(value).expect("valid id")
    }

    let oxygen = id("base:gas/oxygen");
    let mut slot_a_world = WorldGrid::new(GridSize::new(3, 1)).expect("slot_a world");
    slot_a_world
        .set_gas_particles(TilePos::new(1, 0), oxygen, ParticleCount(120))
        .expect("set gas");
    save_game(saves_dir, SCENARIO_GAS_SLOT_A, &slot_a_world, 42, 0).expect("save slot_a");
}

fn count_world_layers(world: &flux_world::WorldGrid) -> (usize, usize, usize) {
    let size = world.size();
    let mut solids = 0usize;
    let mut gas_cells = 0usize;
    for y in 0..size.height {
        for x in 0..size.width {
            let pos = TilePos::new(x, y);
            if world.solid_cell_at(pos).flatten().is_some() {
                solids += 1;
            }
            if world
                .gas_at(pos)
                .map(|gas| gas.total_particles().0)
                .unwrap_or(0)
                > 0
            {
                gas_cells += 1;
            }
        }
    }
    let structures = world.structures().len();
    (solids, gas_cells, structures)
}

#[test]
fn save_load_scenarios_produce_expected_logs_and_semantic_screenshots() {
    let _guard = SAVE_TEST_MUTEX.lock().expect("save mutex");
    let root = workspace_root();
    let saves_dir = root.join("saves");
    let load_artifacts_dir = scenario_artifact_dir("test_scenarios:scenario/save_load_slots");

    let _ = fs::remove_dir_all(load_artifacts_dir);
    let registry = load_registry_for_save_layer_decode();
    generate_slot_saves(&saves_dir, &registry);
    run_flux_app(&["--run-scenario", "test_scenarios:scenario/save_load_slots"]);

    assert!(saves_dir.join("slot_a").join("manifest.json").is_file());
    assert!(saves_dir.join("slot_a").join("payload.bin").is_file());
    assert!(saves_dir.join("slot_b").join("manifest.json").is_file());
    assert!(saves_dir.join("slot_b").join("payload.bin").is_file());

    let lines = read_log_lines("test_scenarios:scenario/save_load_slots");
    assert_substrings_in_order(
        &lines,
        &[
            "scenario validation passed",
            "save load started",
            "step_index=0 step=Log status=ok",
            "step_index=1 step=LoadGame status=ok",
            "step_index=3 step=PauseSimulation status=ok",
            "step_index=4 step=SetCameraPivot status=ok",
            "step_index=6 step=TakeScreenshot status=ok",
            "slot_a_world.png",
            "step_index=9 step=TakeScreenshot status=ok",
            "main_menu_after_slot_a.png",
            "step_index=10 step=Click status=ok",
            "step_index=12 step=PauseSimulation status=ok",
            "step_index=14 step=SetCameraZoom status=ok",
            "step_index=15 step=TakeScreenshot status=ok",
            "slot_b_world.png",
            "step_index=18 step=TakeScreenshot status=ok",
            "main_menu_after_slot_b.png",
            "save load finished",
            "step_index=19 step=Log status=ok",
            "scenario finished: steps=20 final_tick=4",
        ],
    );

    let slot_a = load_game(&saves_dir, "slot_a", &registry).expect("slot_a should load");
    let slot_b = load_game(&saves_dir, "slot_b", &registry).expect("slot_b should load");
    let slot_a_layers = count_world_layers(&slot_a.world);
    let slot_b_layers = count_world_layers(&slot_b.world);
    assert!(slot_a_layers.0 > 0, "slot_a has no solid cells");
    assert!(slot_a_layers.1 > 0, "slot_a has no gas");
    assert!(slot_a_layers.2 > 0, "slot_a has no structures");
    assert!(slot_b_layers.0 > 0, "slot_b has no solid cells");
    assert!(slot_b_layers.1 > 0, "slot_b has no gas");
    assert!(slot_b_layers.2 > 0, "slot_b has no structures");
    assert_ne!(slot_a_layers, slot_b_layers, "slot content should differ");

    let artifact_dir = scenario_artifact_dir("test_scenarios:scenario/save_load_slots");
    let slot_a_world = open_png(&artifact_dir.join("slot_a_world.png"));
    let slot_b_world = open_png(&artifact_dir.join("slot_b_world.png"));
    let menu_after_a = open_png(&artifact_dir.join("main_menu_after_slot_a.png"));
    let menu_after_b = open_png(&artifact_dir.join("main_menu_after_slot_b.png"));

    assert!(slot_a_world.width() >= 640 && slot_a_world.height() >= 360);
    assert_eq!(slot_a_world.dimensions(), slot_b_world.dimensions());
    assert_eq!(menu_after_a.dimensions(), menu_after_b.dimensions());

    let slot_a_content = count_content_pixels(&slot_a_world);
    let slot_b_content = count_content_pixels(&slot_b_world);
    assert!(
        slot_a_content > 3_000,
        "slot_a screenshot looks empty, content pixels={slot_a_content}"
    );
    assert!(
        slot_b_content > 3_000,
        "slot_b screenshot looks empty, content pixels={slot_b_content}"
    );

    let world_diff = pixel_difference_ratio(&slot_a_world, &slot_b_world);
    assert!(
        world_diff > 0.02,
        "slot screenshots should be semantically different, diff_ratio={world_diff}"
    );

    let menu_blue_after_a = count_blue_menu_pixels(&menu_after_a);
    let menu_blue_after_b = count_blue_menu_pixels(&menu_after_b);
    assert!(
        menu_blue_after_a > 2_000,
        "expected visible menu buttons after loading slot_a, blue pixels={menu_blue_after_a}"
    );
    assert!(
        menu_blue_after_b > 2_000,
        "expected visible menu buttons after loading slot_b, blue pixels={menu_blue_after_b}"
    );
}

#[test]
fn gas_diffusion_scenarios_roundtrip_slot_values() {
    let _guard = SAVE_TEST_MUTEX.lock().expect("save mutex");
    let root = workspace_root();
    let saves_dir = root.join("saves");
    let scenario_a = "test_scenarios:scenario/gas_diffusion_slot_a_to_b";
    let scenario_b = "test_scenarios:scenario/gas_diffusion_slot_b_verify";
    let scenario_a_artifacts = scenario_artifact_dir(scenario_a);
    let scenario_b_artifacts = scenario_artifact_dir(scenario_b);

    let registry = load_registry_for_save_layer_decode();
    for backend_policy in ["cpu_only", "prefer_gpu"] {
        let _ = fs::remove_dir_all(&scenario_a_artifacts);
        let _ = fs::remove_dir_all(&scenario_b_artifacts);
        let _ = fs::remove_dir_all(saves_dir.join(SCENARIO_GAS_SLOT_A));
        let _ = fs::remove_dir_all(saves_dir.join(SCENARIO_GAS_SLOT_B));

        generate_diffusion_slot_saves(&saves_dir);
        run_flux_app(&[
            "--run-scenario",
            scenario_a,
            "--backend-policy",
            backend_policy,
        ]);
        run_flux_app(&[
            "--run-scenario",
            scenario_b,
            "--backend-policy",
            backend_policy,
        ]);

        assert_substrings_in_order(
            &read_log_lines(scenario_a),
            &[
                "scenario validation passed",
                "gas diffusion slot_a->slot_b started",
                "step_index=2 step=AssertGasParticlesEq status=ok",
                "step_index=4 step=WaitTicks status=ok",
                "step_index=7 step=AssertGasParticlesEq status=ok",
                "step_index=10 step=SaveGame status=ok",
                "gas diffusion slot_a->slot_b finished",
                "scenario finished: steps=12 final_tick=1",
            ],
        );
        assert_substrings_in_order(
            &read_log_lines(scenario_b),
            &[
                "scenario validation passed",
                "gas diffusion slot_b verify started",
                "step_index=3 step=AssertGasParticlesEq status=ok",
                "step_index=6 step=AssertGasParticlesEq status=ok",
                "gas diffusion slot_b verify finished",
                "scenario finished: steps=8 final_tick=1",
            ],
        );

        let slot_a =
            load_game(&saves_dir, SCENARIO_GAS_SLOT_A, &registry).expect("slot_a should load");
        let slot_b =
            load_game(&saves_dir, SCENARIO_GAS_SLOT_B, &registry).expect("slot_b should load");
        let oxygen = flux_core::PrototypeId::parse("base:gas/oxygen").expect("gas id");
        assert_eq!(
            slot_a.tick, 0,
            "slot_a tick mismatch for backend policy {backend_policy}"
        );
        assert_eq!(
            slot_a
                .world
                .gas_at(TilePos::new(1, 0))
                .expect("cell")
                .particles_of(&oxygen)
                .0,
            120,
            "slot_a center oxygen mismatch for backend policy {backend_policy}"
        );
        assert_eq!(
            slot_b
                .world
                .gas_at(TilePos::new(0, 0))
                .expect("cell")
                .particles_of(&oxygen)
                .0,
            30,
            "slot_b left oxygen mismatch for backend policy {backend_policy}"
        );
        assert_eq!(
            slot_b
                .world
                .gas_at(TilePos::new(1, 0))
                .expect("cell")
                .particles_of(&oxygen)
                .0,
            60,
            "slot_b center oxygen mismatch for backend policy {backend_policy}"
        );
        assert_eq!(
            slot_b
                .world
                .gas_at(TilePos::new(2, 0))
                .expect("cell")
                .particles_of(&oxygen)
                .0,
            30,
            "slot_b right oxygen mismatch for backend policy {backend_policy}"
        );
        assert_eq!(
            slot_b.tick, 1,
            "slot_b tick mismatch for backend policy {backend_policy}"
        );
    }
}

#[test]
fn simulation_scenarios_run_with_cpu_and_prefer_gpu_without_errors() {
    let _guard = SAVE_TEST_MUTEX.lock().expect("save mutex");
    let simulation_scenarios = [
        "test_scenarios:scenario/bootstrap_smoke",
        "test_scenarios:scenario/ui_smoke",
    ];

    for backend_policy in ["cpu_only", "prefer_gpu"] {
        for scenario_id in simulation_scenarios {
            let _ = fs::remove_dir_all(scenario_artifact_dir(scenario_id));
            run_flux_app(&[
                "--run-scenario",
                scenario_id,
                "--backend-policy",
                backend_policy,
            ]);
            let lines = read_log_lines(scenario_id);
            assert!(
                lines.iter().any(|line| line.contains("scenario finished")),
                "scenario `{scenario_id}` did not finish for backend policy `{backend_policy}`"
            );
        }
    }
}
