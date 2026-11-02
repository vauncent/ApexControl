$ErrorActionPreference = 'Stop'
$appRoot = $PSScriptRoot
$env:APEXCONTROL_DATA_DIR = Join-Path $appRoot 'local-data'
$ggRoot = 'C:\Program Files\SteelSeries\GG\'
$serviceName = 'SteelSeriesGGUpdateServiceProxy'
$serviceWasRunning = (Get-Service -Name $serviceName).Status -eq 'Running'
$names = @('SteelSeriesGGEZ.exe','SteelSeriesGG.exe','SteelSeriesGGClient.exe','SteelSeriesEngine.exe','SteelSeriesPrism.exe')
$originalProcesses = @(Get-CimInstance Win32_Process -Filter "Name LIKE 'SteelSeries%'" | Where-Object { $_.Name -in $names })
$result = @{ completed = $false; error = $null; ggRestored = $false }
New-Item -ItemType Directory -Force -Path $env:APEXCONTROL_DATA_DIR | Out-Null
try {
    foreach ($process in $originalProcesses) {
        if (!$process.ExecutablePath -or !$process.ExecutablePath.StartsWith($ggRoot, [StringComparison]::OrdinalIgnoreCase)) {
            throw ('Unexpected executable path; refusing to stop ' + $process.Name)
        }
    }
    if ($serviceWasRunning) { Stop-Service -Name $serviceName }
    $originalProcesses | Sort-Object @{Expression={if ($_.Name -eq 'SteelSeriesGGEZ.exe') {0} else {1}}} | ForEach-Object {
        Stop-Process -Id $_.ProcessId -ErrorAction SilentlyContinue
    }
    Start-Sleep -Seconds 2
    $remaining = @(Get-CimInstance Win32_Process -Filter "Name LIKE 'SteelSeries%'" | Where-Object { $_.Name -in $names })
    if ($remaining.Count) { throw 'GG still running; no device writes attempted' }
    $exe = Join-Path $appRoot 'target\debug\apexcontrol.exe'
    & $exe --exercise-hardware --report (Join-Path $env:APEXCONTROL_DATA_DIR 'hardware-exercise.json') *> (Join-Path $env:APEXCONTROL_DATA_DIR 'hardware-exercise.log')
    $result.testExit = $LASTEXITCODE
    if ($LASTEXITCODE -ne 0) { throw 'Hardware exercise failed; see hardware-exercise.json' }
    $result.completed = $true
} catch {
    $result.error = $_.Exception.Message
} finally {
    try {
        if ($serviceWasRunning) { Start-Service -Name $serviceName }
        if ($originalProcesses.Count) {
            Start-Process -FilePath (Join-Path $ggRoot 'SteelSeriesGGEZ.exe') -WindowStyle Hidden
            $result.ggRestored = $true
        }
    } catch { $result.restoreError = $_.Exception.Message }
    $result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $env:APEXCONTROL_DATA_DIR 'exercise-status.json') -Encoding utf8
}
