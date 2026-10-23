# howtui uninstaller for Windows PowerShell 5.1 and PowerShell 7. Installed next to howtui.exe as
# howtui-uninstall.ps1.
#
#   & "$env:LOCALAPPDATA\Programs\howtui\bin\howtui-uninstall.ps1"           ask, then remove
#   & "$env:LOCALAPPDATA\Programs\howtui\bin\howtui-uninstall.ps1" -Yes      remove without asking
#   & "$env:LOCALAPPDATA\Programs\howtui\bin\howtui-uninstall.ps1" -Purge    also delete the config folder
#
# Removes: the saved sign-in (Credential Manager or file), the shell integration blocks that
# `howtui init powershell --install` added to the PowerShell profiles, howtui.exe, this script, the
# install folder entry in the user PATH and the data folder. Keeps the config folder unless -Purge.
# (ASCII only for Windows PowerShell 5.1.)
param(
    [switch]$Yes,
    [switch]$Purge
)

$ErrorActionPreference = 'Stop'

$dir = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $dir 'howtui.exe'
if (-not (Test-Path -LiteralPath $exe)) {
    $cmd = Get-Command -Name howtui -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    $exe = if ($cmd) { $cmd.Path } else { $null }
}
$data = if ($env:HOWTUI_DATA_DIR) { $env:HOWTUI_DATA_DIR } else { Join-Path $env:LOCALAPPDATA 'howtui' }
$config = Join-Path $env:APPDATA 'howtui'
$self = Join-Path $dir 'howtui-uninstall.ps1'

Write-Host 'This removes howtui:'
Write-Host '  - saved sign-in (signs out and deletes the tokens)'
Write-Host '  - shell integration blocks in your PowerShell profiles (only the marked blocks)'
if ($exe) { Write-Host "  - $exe" }
Write-Host "  - $self"
Write-Host "  - $dir from your user PATH"
Write-Host "  - $data"
if ($Purge) { Write-Host "  - $config" } elseif (Test-Path -LiteralPath $config) { Write-Host "Kept: $config (use -Purge to delete it)" }

if (-not $Yes) {
    $interactive = [Environment]::UserInteractive -and -not [Console]::IsInputRedirected -and $env:HOWTUI_TEST_NO_TTY -ne '1'
    if (-not $interactive) {
        [Console]::Error.WriteLine('howtui uninstall: no console to confirm; run again with -Yes')
        exit 1
    }
    $answer = Read-Host 'Continue? [y/N]'
    if ($answer -notmatch '^(y|yes)$') {
        Write-Host 'Nothing removed.'
        return
    }
}

if ($exe) {
    # keep going on failures (offline, already signed out)
    try { & $exe logout --forget | Out-Host } catch { }
    try { & $exe init powershell --uninstall | Out-Host } catch { }
    Remove-Item -LiteralPath $exe -Force -ErrorAction SilentlyContinue
}

# user PATH: read raw (no %VAR% expansion), drop this folder, write back as REG_EXPAND_SZ
$key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
try {
    $current = $key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    $entries = @($current -split ';' | Where-Object { $_ -ne '' })
    $kept = @($entries | Where-Object { $_.TrimEnd('\') -ine $dir.TrimEnd('\') })
    if ($kept.Count -ne $entries.Count) {
        $key.SetValue('Path', ($kept -join ';'), [Microsoft.Win32.RegistryValueKind]::ExpandString)
        Write-Host "Removed $dir from the user PATH."
    }
} finally {
    $key.Close()
}

# only files howtui creates, then empty folders (a wrong HOWTUI_DATA_DIR never loses other files)
foreach ($f in 'state.json', 'credentials.json', 'auth.lock', 'logs\howtui.log') {
    Remove-Item -LiteralPath (Join-Path $data $f) -Force -ErrorAction SilentlyContinue
}
foreach ($d in (Join-Path $data 'logs'), $data) {
    if ((Test-Path -LiteralPath $d) -and -not (Get-ChildItem -LiteralPath $d -Force)) { Remove-Item -LiteralPath $d -Force }
}
if ($Purge) {
    Remove-Item -LiteralPath (Join-Path $config 'config.toml') -Force -ErrorAction SilentlyContinue
    if ((Test-Path -LiteralPath $config) -and -not (Get-ChildItem -LiteralPath $config -Force)) { Remove-Item -LiteralPath $config -Force }
}
Remove-Item -LiteralPath $self -Force -ErrorAction SilentlyContinue
if ((Test-Path -LiteralPath $dir) -and -not (Get-ChildItem -LiteralPath $dir -Force)) { Remove-Item -LiteralPath $dir -Force }
Write-Host 'howtui removed. Open a new terminal so the shell integration is gone from this session too.'
