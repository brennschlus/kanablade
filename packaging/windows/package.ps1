# Builds dist/kanablade-<version>-windows-x86_64.zip: the .exe (icon and version
# info embedded by build.rs, no console window) and the font license.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..\..')

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches.Groups[1].Value
$name = "kanablade-$version-windows-x86_64"

cargo build --release --locked
if ($LASTEXITCODE) { exit $LASTEXITCODE }

$root = "dist\$name"
Remove-Item -Recurse -Force $root, "dist\$name.zip" -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $root | Out-Null
Copy-Item target\release\kanablade.exe, assets\fonts\OFL-ZenFonts.txt $root
Compress-Archive -Path "$root\*" -DestinationPath "dist\$name.zip"
"dist\$name.zip"
