# install.ps1 verification (TC-INS-005, TC-INS-003): local fake release over HTTP, install, latest lookup,
# tamper detection, user PATH registration as REG_EXPAND_SZ. Run under both powershell (5.1) and pwsh (7).
# Usage: tests/install/run_windows.ps1 -Exe target\release\howtui.exe
param([Parameter(Mandatory = $true)][string]$Exe)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$version = ((& $Exe --version) | Out-String).Trim().Split(' ')[1]
$cpu = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64' -or $env:PROCESSOR_ARCHITEW6432 -eq 'ARM64') { 'aarch64' } else { 'x86_64' }
$asset = "howtui-$version-$cpu-pc-windows-msvc.zip"
$work = Join-Path ([IO.Path]::GetTempPath()) ('howtui-ps-test-' + [Guid]::NewGuid().ToString('N'))
$dist = Join-Path $work 'dist'
New-Item -ItemType Directory -Path (Join-Path $dist "v$version"), (Join-Path $dist 'latest'), (Join-Path $work 'pkg') | Out-Null
Copy-Item $Exe (Join-Path $work 'pkg\howtui.exe')
Copy-Item (Join-Path $root 'README.md'), (Join-Path $root 'LICENSE') (Join-Path $work 'pkg')
Compress-Archive -Path (Join-Path $work 'pkg\*') -DestinationPath (Join-Path $dist "v$version\$asset")
$hash = (Get-FileHash (Join-Path $dist "v$version\$asset") -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -Path (Join-Path $dist "v$version\SHA256SUMS") -Value "$hash  $asset" -Encoding ascii
Copy-Item (Join-Path $dist "v$version\SHA256SUMS") (Join-Path $dist 'latest\SHA256SUMS')

$port = 18765
$server = Start-Process -FilePath python -ArgumentList '-m', 'http.server', $port, '--bind', '127.0.0.1', '--directory', $dist -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 2
$failed = $false
function Fail([string]$m) { Write-Host "FAIL $m"; $script:failed = $true }
$script = Get-Content -Raw -LiteralPath (Join-Path $root 'scripts\install.ps1')
try {
    $env:HOWTUI_DOWNLOAD_BASE = "http://127.0.0.1:$port"
    $env:HOWTUI_LATEST_BASE = "http://127.0.0.1:$port/latest"
    $env:HOWTUI_INSTALL_DIR = Join-Path $work 'bin'
    $env:HOWTUI_NO_MODIFY_PATH = '1'
    Remove-Item Env:HOWTUI_VERSION -ErrorAction SilentlyContinue

    # irm | iex 와 같은 방식으로 실행 (latest 조회 경로)
    Invoke-Expression $script
    $installed = ((& (Join-Path $env:HOWTUI_INSTALL_DIR 'howtui.exe') --version) | Out-String).Trim()
    if ($installed -ne "howtui $version") { Fail "installed version: $installed" } else { Write-Host "PASS install via latest ($($PSVersionTable.PSVersion))" }

    # 사용자 PATH 등록 (REG_EXPAND_SZ)
    Remove-Item Env:HOWTUI_NO_MODIFY_PATH
    $env:HOWTUI_VERSION = $version
    Invoke-Expression $script
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment')
    $kind = $key.GetValueKind('Path')
    $raw = $key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    $key.Close()
    if ($raw -notlike "*$($env:HOWTUI_INSTALL_DIR)*" -or $kind -ne 'ExpandString') { Fail "user PATH ($kind)" } else { Write-Host 'PASS user PATH as ExpandString' }

    # 변조 감지: 설치 중단, 기존 바이너리 유지
    Add-Content -LiteralPath (Join-Path $dist "v$version\$asset") -Value 'tampered'
    $threw = $false
    try { Invoke-Expression $script } catch { $threw = $_.Exception.Message -like '*checksum mismatch*' }
    if (-not $threw) { Fail 'tampered archive accepted' } else { Write-Host 'PASS tamper detection' }
    $still = ((& (Join-Path $env:HOWTUI_INSTALL_DIR 'howtui.exe') --version) | Out-String).Trim()
    if ($still -ne "howtui $version") { Fail 'existing binary damaged' }
} finally {
    Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
if ($failed) { exit 1 }
