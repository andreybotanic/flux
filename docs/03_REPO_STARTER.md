# Repo starter files

Этот архив содержит файлы, которые нужно положить в пустой репозиторий перед началом staged-разработки.

## 1. Что уже входит

Root files:

```text
Cargo.toml
rust-toolchain.toml
rustfmt.toml
.gitignore
.gitattributes
.editorconfig
AGENTS.md
README.md
LICENSE.md
```

CI/hooks/scripts:

```text
.github/workflows/ci.yml
.github/pull_request_template.md
.githooks/pre-commit
.githooks/pre-push
scripts/ci.sh
scripts/install_git_hooks.sh
scripts/install_git_hooks.ps1
scripts/check_plan_index.py
```

Starter crates:

```text
crates/flux_core
crates/flux_app
```

Docs:

```text
docs/00_OVERVIEW.md
docs/01_ROADMAP_DAG.md
docs/02_PROJECT_CONVENTIONS.md
docs/stages/*.md
```

## 2. Что сделать вручную после распаковки

```bash
git init
git add .
git commit -m "Initial FluxEngine starter"
./scripts/install_git_hooks.sh
./scripts/ci.sh
# On Windows CMD/PowerShell:
scripts\ci.cmd
```

## 3. GitHub repository settings

Рекомендуется включить:

- protected `main`;
- squash merge only;
- запрет merge commits;
- обязательный CI перед merge;
- один PR = один stage.

## 4. Что НЕ делать вручную

Не нужно заранее:

- добавлять Bevy;
- делать UI;
- делать модовый loader;
- делать физику;
- делать GPU;
- создавать `base` mod.

Это делается строго по stage-документам.

## Windows / PowerShell execution policy

Если PowerShell блокирует `scripts/install_git_hooks.ps1` из-за отсутствия цифровой подписи, не меняй системную execution policy. Используй один из вариантов:

```cmd
scripts\install_git_hooks.cmd
scripts\ci.cmd
```

или вручную:

```cmd
git config core.hooksPath .githooks
git config --local --get core.hooksPath
```

Разовый обход PowerShell policy тоже допустим, но не обязателен:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install_git_hooks.ps1
```
