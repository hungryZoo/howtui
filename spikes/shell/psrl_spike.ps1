# SPIKE-05 PowerShell PSReadLine 입력 버퍼 삽입 검증용 스크립트. (Linux pwsh 7 컨테이너에서 API 동작만 확인)
# 사용: . ./psrl_spike.ps1  (HOWTUI_SPIKE_BIN, HOWTUI_SPIKE_LOG 환경변수 필요)
Set-PSReadLineOption -HistorySaveStyle SaveNothing
Set-PSReadLineKeyHandler -Chord 'Ctrl+g' -ScriptBlock {
    $line = $null; $cursor = $null
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)
    $out = & $env:HOWTUI_SPIKE_BIN --quiet
    $rc = $LASTEXITCODE
    $text = ''
    if ($rc -eq 0 -and $out) {
        $text = ($out -join "`n")
        [Microsoft.PowerShell.PSConsoleReadLine]::Replace(0, $line.Length, $text)
    }
    [Microsoft.PowerShell.PSConsoleReadLine]::InvokePrompt()
    Add-Content -Path $env:HOWTUI_SPIKE_LOG -Value "widget_rc=$rc before_len=$($line.Length) after_len=$($text.Length) ps=$($PSVersionTable.PSVersion) psrl=$((Get-Module PSReadLine).Version)"
}
function prompt { 'PS> ' }
