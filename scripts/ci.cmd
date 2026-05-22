@echo off
setlocal

cargo fmt --all --check
if errorlevel 1 exit /b %errorlevel%

cargo clippy --workspace --all-targets -- -D warnings
if errorlevel 1 exit /b %errorlevel%

cargo test --workspace
if errorlevel 1 exit /b %errorlevel%

python scripts\check_plan_index.py
if errorlevel 1 goto :python_fallback
goto :done

:python_fallback
py -3 scripts\check_plan_index.py
if errorlevel 1 (
  echo ERROR: neither "python" nor "py -3" could run scripts\check_plan_index.py
  exit /b %errorlevel%
)

:done

endlocal
