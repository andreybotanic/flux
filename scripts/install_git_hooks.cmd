@echo off
setlocal

git config core.hooksPath .githooks
if errorlevel 1 exit /b %errorlevel%

echo Git hooks installed via core.hooksPath=.githooks
git config --local --get core.hooksPath

endlocal
