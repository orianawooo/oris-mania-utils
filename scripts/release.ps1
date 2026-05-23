$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $repoRoot

$package = Get-Content (Join-Path $repoRoot 'package.json') | ConvertFrom-Json
$version = $package.version
$distDir = Join-Path $repoRoot 'dist'
$exePath = Join-Path $repoRoot 'src-tauri\target\release\oris-mania-utils.exe'
$msiDir = Join-Path $repoRoot 'src-tauri\target\release\bundle\msi'

npm run check
npm run build:desktop
npm run smoke

if (!(Test-Path $exePath)) {
    throw "Missing release executable at $exePath"
}

New-Item -ItemType Directory -Force -Path $distDir | Out-Null

$zipPath = Join-Path $distDir ("oris-mania-utils-v{0}.zip" -f $version)
if (Test-Path $zipPath) {
    Remove-Item $zipPath -Force
}

$tempDir = Join-Path $env:TEMP ("oris-mania-utils-release-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $tempDir | Out-Null
Copy-Item $exePath (Join-Path $tempDir 'oris-mania-utils.exe') -Force
Compress-Archive -Path (Join-Path $tempDir '*') -DestinationPath $zipPath
Remove-Item $tempDir -Recurse -Force

$latestMsi = Get-ChildItem $msiDir -Filter *.msi | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if ($latestMsi) {
    Copy-Item $latestMsi.FullName (Join-Path $distDir $latestMsi.Name) -Force
}

Write-Host "Release artifacts ready:"
Write-Host "  ZIP: $zipPath"
if ($latestMsi) {
    Write-Host "  MSI: $(Join-Path $distDir $latestMsi.Name)"
}
