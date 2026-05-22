# FluxEngine

FluxEngine — 2D simulation game/platform written in Rust. The project starts from a strict staged architecture: mods, DLC, content registries, scenario automation, world data layout, UI extension points and GPU backend contracts are designed before full gameplay implementation.

## Start here

Read these files before changing code:

1. `AGENTS.md`
2. `docs/00_OVERVIEW.md`
3. `docs/01_ROADMAP_DAG.md`
4. `docs/02_PROJECT_CONVENTIONS.md`
5. the current stage task from `docs/stages/`

## Initial local setup

```bash
git init
git add .
git commit -m "Initial FluxEngine starter"
./scripts/install_git_hooks.sh
./scripts/ci.sh
```

On Windows PowerShell:

```powershell
git init
git add .
git commit -m "Initial FluxEngine starter"
./scripts/install_git_hooks.ps1
./scripts/ci.cmd
```

## Current bootstrap state

This archive already contains a minimal Rust workspace with two placeholder crates:

- `flux_core`
- `flux_app`

Stage `S00_REPO_BOOTSTRAP` should verify and adopt this starter state rather than recreate it blindly.

## Windows note

PowerShell may block unsigned `.ps1` scripts. In that case use the CMD installer instead:

```cmd
scripts\install_git_hooks.cmd
scripts\ci.cmd
```

Or install hooks manually:

```cmd
git config core.hooksPath .githooks
git config --local --get core.hooksPath
```
