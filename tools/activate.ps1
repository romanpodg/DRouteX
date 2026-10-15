# Dot-source this file: . ./tools/activate.ps1
# Changes only the current PowerShell process; no persistent PATH edits.
$routiraRoot = Split-Path -Parent $PSScriptRoot
$env:CARGO_HOME = Join-Path $routiraRoot '.tools/cargo'
$env:RUSTUP_HOME = Join-Path $routiraRoot '.tools/rustup'
$routiraBin = Join-Path $env:CARGO_HOME 'bin'
$routiraAuxBin = Join-Path $routiraRoot '.tools/bin'
# Rust's MSVC host needs the existing Visual Studio native developer environment.
$routiraVswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
if ((Test-Path -LiteralPath $routiraVswhere) -and -not $env:VSCMD_VER) {
    $routiraVs = & $routiraVswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($routiraVs) {
        Import-Module (Join-Path $routiraVs 'Common7/Tools/Microsoft.VisualStudio.DevShell.dll')
        Enter-VsDevShell -VsInstallPath $routiraVs -SkipAutomaticLocation -Arch amd64 -HostArch amd64 | Out-Null
    }
}
foreach ($routiraPath in @($routiraBin, $routiraAuxBin, (Join-Path $env:USERPROFILE '.local/bin'), (Join-Path $routiraRoot '.tools/codescene'))) {
    if (($env:PATH -split ';') -notcontains $routiraPath) { $env:PATH = "$routiraPath;$env:PATH" }
}
# Portable Microsoft SDK libraries/headers supplement the existing MSVC compiler.
$routiraSdkLib = Join-Path $routiraRoot '.tools/windows-sdk-x64/c'
$routiraSdkInclude = Join-Path $routiraRoot '.tools/windows-sdk-cpp/c/Include/10.0.28000.0'
foreach ($routiraPart in @('um/x64', 'ucrt/x64')) {
    $routiraPath = Join-Path $routiraSdkLib $routiraPart
    if ((Test-Path -LiteralPath $routiraPath) -and (($env:LIB -split ';') -notcontains $routiraPath)) { $env:LIB = "$routiraPath;$env:LIB" }
}
foreach ($routiraPart in @('shared', 'um', 'ucrt', 'winrt')) {
    $routiraPath = Join-Path $routiraSdkInclude $routiraPart
    if ((Test-Path -LiteralPath $routiraPath) -and (($env:INCLUDE -split ';') -notcontains $routiraPath)) { $env:INCLUDE = "$routiraPath;$env:INCLUDE" }
}
Remove-Variable routiraSdkLib, routiraSdkInclude, routiraPart
Remove-Variable routiraVswhere, routiraVs -ErrorAction SilentlyContinue
Remove-Variable routiraRoot, routiraBin, routiraAuxBin, routiraPath
