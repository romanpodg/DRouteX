# Dot-source this file: . ./tools/activate.ps1
# Changes only the current PowerShell process; no persistent PATH edits.
$droutexRoot = Split-Path -Parent $PSScriptRoot
$env:CARGO_HOME = Join-Path $droutexRoot '.tools/cargo'
$env:RUSTUP_HOME = Join-Path $droutexRoot '.tools/rustup'
$droutexBin = Join-Path $env:CARGO_HOME 'bin'
$droutexAuxBin = Join-Path $droutexRoot '.tools/bin'
# Rust's MSVC host needs the existing Visual Studio native developer environment.
$droutexVswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
if ((Test-Path -LiteralPath $droutexVswhere) -and -not $env:VSCMD_VER) {
    $droutexVs = & $droutexVswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($droutexVs) {
        Import-Module (Join-Path $droutexVs 'Common7/Tools/Microsoft.VisualStudio.DevShell.dll')
        Enter-VsDevShell -VsInstallPath $droutexVs -SkipAutomaticLocation -Arch amd64 -HostArch amd64 | Out-Null
    }
}
foreach ($droutexPath in @($droutexBin, $droutexAuxBin, (Join-Path $env:USERPROFILE '.local/bin'), (Join-Path $droutexRoot '.tools/codescene'))) {
    if (($env:PATH -split ';') -notcontains $droutexPath) { $env:PATH = "$droutexPath;$env:PATH" }
}
# Portable Microsoft SDK libraries/headers supplement the existing MSVC compiler.
$droutexSdkLib = Join-Path $droutexRoot '.tools/windows-sdk-x64/c'
$droutexSdkInclude = Join-Path $droutexRoot '.tools/windows-sdk-cpp/c/Include/10.0.28000.0'
foreach ($droutexPart in @('um/x64', 'ucrt/x64')) {
    $droutexPath = Join-Path $droutexSdkLib $droutexPart
    if ((Test-Path -LiteralPath $droutexPath) -and (($env:LIB -split ';') -notcontains $droutexPath)) { $env:LIB = "$droutexPath;$env:LIB" }
}
foreach ($droutexPart in @('shared', 'um', 'ucrt', 'winrt')) {
    $droutexPath = Join-Path $droutexSdkInclude $droutexPart
    if ((Test-Path -LiteralPath $droutexPath) -and (($env:INCLUDE -split ';') -notcontains $droutexPath)) { $env:INCLUDE = "$droutexPath;$env:INCLUDE" }
}
Remove-Variable droutexSdkLib, droutexSdkInclude, droutexPart
Remove-Variable droutexVswhere, droutexVs -ErrorAction SilentlyContinue
Remove-Variable droutexRoot, droutexBin, droutexAuxBin, droutexPath
