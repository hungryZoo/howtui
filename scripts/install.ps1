# howtui installer for Windows PowerShell 5.1 and PowerShell 7 (SRS 14.3). No administrator rights required.
#
#   irm https://raw.githubusercontent.com/hungryZoo/howtui/main/scripts/install.ps1 | iex
#
# Review first:
#   irm https://raw.githubusercontent.com/hungryZoo/howtui/main/scripts/install.ps1 -OutFile install.ps1
#   notepad install.ps1; powershell -ExecutionPolicy Bypass -File .\install.ps1
#
# Environment:
#   HOWTUI_VERSION          install a specific release (default: latest)
#   HOWTUI_INSTALL_DIR      install directory (default: %LOCALAPPDATA%\Programs\howtui\bin)
#   HOWTUI_NO_MODIFY_PATH   set to 1 to skip adding the directory to the user PATH
#   HOWTUI_ARCH             override the detected architecture (x86_64 or aarch64)
#
# This script only downloads the release archive and SHA256SUMS of this repository, verifies the
# checksum, and copies howtui.exe. It never edits PowerShell profiles. (ASCII only for 5.1.)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Install-Howtui {
    $repo = 'hungryZoo/howtui'
    $base = if ($env:HOWTUI_DOWNLOAD_BASE) { $env:HOWTUI_DOWNLOAD_BASE.TrimEnd('/') } else { "https://github.com/$repo/releases/download" }
    $latest = if ($env:HOWTUI_LATEST_BASE) { $env:HOWTUI_LATEST_BASE.TrimEnd('/') } else { "https://github.com/$repo/releases/latest/download" }

    # TLS 1.2 for Windows PowerShell 5.1 (keep the OS default when it is SystemDefault)
    if ([int][Net.ServicePointManager]::SecurityProtocol -ne 0) {
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor 3072
    }

    $arch = $env:HOWTUI_ARCH
    if (-not $arch) {
        try {
            $arch = (Get-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment' -Name PROCESSOR_ARCHITECTURE).PROCESSOR_ARCHITECTURE
        } catch { $arch = $null }
    }
    if (-not $arch) { $arch = $env:PROCESSOR_ARCHITEW6432 }
    if (-not $arch) { $arch = $env:PROCESSOR_ARCHITECTURE }
    switch -Regex ($arch) {
        '^(ARM64|aarch64)$' { $cpu = 'aarch64' }
        '^(AMD64|x86_64|x64)$' { $cpu = 'x86_64' }
        default { throw "howtui install: unsupported architecture: $arch" }
    }
    $target = "$cpu-pc-windows-msvc"
    Write-Host "==> target: $target"

    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("howtui-install-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        $version = $env:HOWTUI_VERSION
        if ($version) {
            $version = $version.TrimStart('v')
        } else {
            $sums = Join-Path $tmp 'SHA256SUMS.latest'
            Invoke-WebRequest -UseBasicParsing -Uri "$latest/SHA256SUMS" -OutFile $sums
            foreach ($line in Get-Content -LiteralPath $sums) {
                if ($line -match "howtui-(\d+\.\d+\.\d+)-$([regex]::Escape($target))\.zip") { $version = $Matches[1]; break }
            }
            if (-not $version) { throw "howtui install: could not determine the latest version" }
        }
        Write-Host "==> version: $version"

        $asset = "howtui-$version-$target.zip"
        $zip = Join-Path $tmp $asset
        $sumFile = Join-Path $tmp 'SHA256SUMS'
        Write-Host "==> downloading $asset"
        Invoke-WebRequest -UseBasicParsing -Uri "$base/v$version/$asset" -OutFile $zip
        Invoke-WebRequest -UseBasicParsing -Uri "$base/v$version/SHA256SUMS" -OutFile $sumFile

        $expected = $null
        foreach ($line in Get-Content -LiteralPath $sumFile) {
            $parts = $line -split '\s+', 2
            if ($parts.Count -eq 2 -and $parts[1].TrimStart('*').Trim() -eq $asset) { $expected = $parts[0].Trim(); break }
        }
        if (-not $expected) { throw "howtui install: SHA256SUMS has no entry for $asset" }
        $actual = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash
        if ($actual -ine $expected) { throw "howtui install: checksum mismatch for $asset (expected $expected, got $actual)" }
        Write-Host '==> checksum verified'

        $extract = Join-Path $tmp 'x'
        Expand-Archive -LiteralPath $zip -DestinationPath $extract -Force
        $exe = Join-Path $extract 'howtui.exe'
        if (-not (Test-Path -LiteralPath $exe)) { throw 'howtui install: archive does not contain howtui.exe' }

        $dir = if ($env:HOWTUI_INSTALL_DIR) { $env:HOWTUI_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\howtui\bin' }
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        $dest = Join-Path $dir 'howtui.exe'
        $staged = Join-Path $dir 'howtui.exe.new'
        Copy-Item -LiteralPath $exe -Destination $staged -Force
        try {
            Move-Item -LiteralPath $staged -Destination $dest -Force
        } catch {
            Remove-Item -LiteralPath $staged -Force -ErrorAction SilentlyContinue
            throw "howtui install: cannot replace $dest (is howtui running?)"
        }

        $installed = (& $dest --version) | Out-String
        if ($installed.Trim() -ne "howtui $version") { throw "howtui install: installed binary reports '$($installed.Trim())'" }
        Write-Host "==> installed $($installed.Trim()) to $dest"

        if ($env:HOWTUI_NO_MODIFY_PATH -eq '1') {
            Write-Host "Add $dir to your PATH to run howtui."
        } else {
            Add-HowtuiUserPath -Dir $dir
        }
        Write-Host ''
        Write-Host 'Run: howtui'
    } finally {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Add-HowtuiUserPath {
    param([string]$Dir)
    # Read the raw value (no %VAR% expansion) and write it back as REG_EXPAND_SZ; never use setx.
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
    try {
        $current = $key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        $entries = @($current -split ';' | Where-Object { $_ -ne '' })
        if ($entries | Where-Object { $_.TrimEnd('\') -ieq $Dir.TrimEnd('\') }) {
            Write-Host "$Dir is already on the user PATH."
        } else {
            $new = (@($entries) + $Dir) -join ';'
            $key.SetValue('Path', $new, [Microsoft.Win32.RegistryValueKind]::ExpandString)
            Send-HowtuiSettingChange
            Write-Host "Added $Dir to the user PATH. Open a new terminal to use it."
        }
    } finally {
        $key.Close()
    }
    if (-not (($env:Path -split ';') | Where-Object { $_.TrimEnd('\') -ieq $Dir.TrimEnd('\') })) {
        $env:Path = "$Dir;$env:Path"
    }
}

function Send-HowtuiSettingChange {
    if (-not ('Howtui.Native' -as [type])) {
        Add-Type -Namespace 'Howtui' -Name 'Native' -MemberDefinition @'
[System.Runtime.InteropServices.DllImport("user32.dll", SetLastError = true, CharSet = System.Runtime.InteropServices.CharSet.Auto)]
public static extern System.IntPtr SendMessageTimeout(System.IntPtr hWnd, uint Msg, System.UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out System.UIntPtr lpdwResult);
'@
    }
    $result = [UIntPtr]::Zero
    # HWND_BROADCAST = 0xffff, WM_SETTINGCHANGE = 0x1a, SMTO_ABORTIFHUNG = 0x2
    [void][Howtui.Native]::SendMessageTimeout([IntPtr]0xffff, 0x1a, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$result)
}

Install-Howtui
