$ErrorActionPreference = "Stop"

git config core.hooksPath .githooks
Write-Host "Git hooks installed via core.hooksPath=.githooks"
Write-Host "On Windows, hooks will run through Git Bash if available."
