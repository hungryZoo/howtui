# howtui shell integration for PowerShell 7 and Windows PowerShell 5.1 (key binding needs PSReadLine 2.0.0 or newer).
#
# Enable:  add this line to your profile (notepad $PROFILE), then open a new session
#            howtui init powershell | Out-String | Invoke-Expression
# Disable: remove that line and open a new session.
# Options: howtui init powershell --key alt+h    use another key (default ctrl+g)
#          howtui init powershell --no-bind      do not bind a key
#          howtui init powershell --no-wrap      do not define the howtui function
#
# @KEY_NAME@ at the prompt: open howtui with the current line; the accepted command replaces the line.
# Typing howtui at the prompt: the accepted command is placed in the next prompt line and in history.
# Nothing is executed automatically. This script never edits your profile. (ASCII only for 5.1.)

function global:__howtui_exe {
    # resolved at call time: the executable on PATH, never this function
    $cmd = Get-Command -Name howtui -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($cmd) { $cmd.Path } else { $null }
}

function global:__howtui_read([string]$Path) {
    try {
        $text = [System.IO.File]::ReadAllText($Path, (New-Object System.Text.UTF8Encoding $false))
    } catch {
        return ''
    }
    return ($text -replace "`r`n", "`n")
}

#@wrap
function global:__howtui_next([string]$Text) {
    if (-not (Get-Module -Name PSReadLine)) { return }
    [Microsoft.PowerShell.PSConsoleReadLine]::AddToHistory($Text)
    $global:__howtui_pending = $Text
    # PSReadLine raises OnIdle while it waits for input at the next prompt
    $null = Register-EngineEvent -SourceIdentifier PowerShell.OnIdle -MaxTriggerCount 1 -Action {
        $t = $global:__howtui_pending
        Remove-Variable -Name __howtui_pending -Scope Global -ErrorAction SilentlyContinue
        $line = $null; $cursor = $null
        [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)
        if ($t -and $line.Length -eq 0) { [Microsoft.PowerShell.PSConsoleReadLine]::Insert($t) }
    }
}

function global:howtui {
    $exe = __howtui_exe
    if (-not $exe) { Write-Error 'howtui: the howtui executable was not found on PATH'; return }
    $file = [System.IO.Path]::GetTempFileName()
    $names = 'HOWTUI_SHELL', 'HOWTUI_SHELL_VERSION', 'HOWTUI_SHELL_INTEGRATION', 'HOWTUI_NEXT_INPUT_FILE'
    $saved = @{}
    foreach ($n in $names) { $saved[$n] = [Environment]::GetEnvironmentVariable($n) }
    $env:HOWTUI_SHELL = 'powershell'
    $env:HOWTUI_SHELL_VERSION = [string]$PSVersionTable.PSVersion
    $env:HOWTUI_SHELL_INTEGRATION = 'powershell'
    $env:HOWTUI_NEXT_INPUT_FILE = $file
    try {
        # output that is captured or piped stays on stdout (the executable checks its stdout)
        & $exe @args
        $rc = $LASTEXITCODE
    } finally {
        foreach ($n in $names) { [Environment]::SetEnvironmentVariable($n, $saved[$n]) }
    }
    $text = __howtui_read $file
    Remove-Item -LiteralPath $file -Force -ErrorAction SilentlyContinue
    if ($rc -eq 0 -and $text) { __howtui_next $text }
    $global:LASTEXITCODE = $rc
}
#@end-wrap

#@bind
function global:__howtui_widget {
    $line = $null; $cursor = $null
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)
    $exe = __howtui_exe
    if (-not $exe) { [Microsoft.PowerShell.PSConsoleReadLine]::Ding(); return }
    $file = [System.IO.Path]::GetTempFileName()
    # no redirection: the TUI needs the console (the call operator would redirect stdout here)
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $exe
    $psi.Arguments = '--widget'
    $psi.UseShellExecute = $false
    $psi.EnvironmentVariables['HOWTUI_SHELL'] = 'powershell'
    $psi.EnvironmentVariables['HOWTUI_SHELL_VERSION'] = [string]$PSVersionTable.PSVersion
    $psi.EnvironmentVariables['HOWTUI_SHELL_INTEGRATION'] = 'powershell'
    $psi.EnvironmentVariables['HOWTUI_BUFFER'] = $line
    $psi.EnvironmentVariables['HOWTUI_LBUFFER'] = $line.Substring(0, $cursor)
    $psi.EnvironmentVariables['HOWTUI_OUTPUT_FILE'] = $file
    $rc = 1
    try {
        $p = [System.Diagnostics.Process]::Start($psi)
        $p.WaitForExit()
        $rc = $p.ExitCode
    } catch {
        $rc = 1
    }
    # prompt row for the redraw: the TUI has put the cursor back on the input line (moved up if the
    # screen scrolled); taken before the line changes
    $y = [Console]::CursorTop - (Get-PSReadLineOption).ExtraPromptLineCount
    $text = __howtui_read $file
    Remove-Item -LiteralPath $file -Force -ErrorAction SilentlyContinue
    if ($rc -eq 0 -and $text) {
        if ($line.Length -gt 0) {
            [Microsoft.PowerShell.PSConsoleReadLine]::Replace(0, $line.Length, $text)
        } else {
            [Microsoft.PowerShell.PSConsoleReadLine]::Insert($text)
        }
        [Microsoft.PowerShell.PSConsoleReadLine]::SetCursorPosition($text.Length)
    }
    if ($y -lt 0) { $y = 0 }
    if ($y -gt [Console]::BufferHeight - 1) { $y = [Console]::BufferHeight - 1 }
    [Microsoft.PowerShell.PSConsoleReadLine]::InvokePrompt($null, $y)
}

$__howtui_psrl = Get-Module -Name PSReadLine
if (-not $__howtui_psrl) {
    Write-Warning 'howtui: PSReadLine is not loaded; the @KEY_NAME@ binding was skipped'
} elseif ($__howtui_psrl.Version -lt [version]'2.0.0') {
    Write-Warning "howtui: the @KEY_NAME@ binding needs PSReadLine 2.0.0 or newer (this is $($__howtui_psrl.Version)); skipped"
} else {
    $__howtui_bind = @{
        Chord = '@KEY_PS@'
        BriefDescription = 'howtui'
        Description = 'Open howtui with the current command line'
        ScriptBlock = { __howtui_widget }
    }
    if ((Get-PSReadLineOption).EditMode -eq 'Vi') {
        Set-PSReadLineKeyHandler @__howtui_bind -ViMode Insert
        Set-PSReadLineKeyHandler @__howtui_bind -ViMode Command
    } else {
        Set-PSReadLineKeyHandler @__howtui_bind
    }
    Remove-Variable -Name __howtui_bind
}
Remove-Variable -Name __howtui_psrl
#@end-bind
