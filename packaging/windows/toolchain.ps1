param(
    [ValidateSet("amd64", "arm64")]
    [string]$Architecture = "amd64",
    [ValidateSet("x86.x64", "ARM64")]
    [string]$Component = "x86.x64"
)

$ErrorActionPreference = "Stop"
$before = @{}
Get-ChildItem Env: | ForEach-Object { $before[$_.Name] = $_.Value }
$vswhere = "${env:ProgramFiles(x86)}/Microsoft Visual Studio/Installer/vswhere.exe"
$installation = & $vswhere -latest -products '*' -requires "Microsoft.VisualStudio.Component.VC.Tools.$Component" -property installationPath
if (-not $installation) { throw "Visual Studio C++ toolchain not found" }
& "$installation/Common7/Tools/Launch-VsDevShell.ps1" -SkipAutomaticLocation -Arch $Architecture -HostArch amd64
$env:GPUI_FXC_PATH = (Get-Command fxc.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
Get-ChildItem Env: | Where-Object { $before[$_.Name] -ne $_.Value } | ForEach-Object {
    "$($_.Name)=$($_.Value)" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8
}
