@echo off
setlocal

cargo fmt --all --check
if errorlevel 1 exit /b %errorlevel%

cargo clippy --workspace --all-targets -- -D warnings
if errorlevel 1 exit /b %errorlevel%

cargo test --workspace
if errorlevel 1 exit /b %errorlevel%

python scripts\check_plan_index.py
if errorlevel 1 exit /b %errorlevel%

endlocal
