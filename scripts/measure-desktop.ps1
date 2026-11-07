param(
    [Parameter(Mandatory=$true)][int]$BoboProcessId,
    [string]$Label = 'debug',
    [ValidateRange(1,60)][int]$Seconds = 10
)

# Include only this application's process tree, including WebView2 subprocesses.
function Get-BoboSample {
    $processes = Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId
    $ids = [System.Collections.Generic.HashSet[int]]::new()
    $null = $ids.Add($BoboProcessId)
    do {
        $added = $false
        foreach ($process in $processes) {
            if ($ids.Contains([int]$process.ParentProcessId) -and $ids.Add([int]$process.ProcessId)) { $added = $true }
        }
    } while ($added)
    $sample = @{}
    foreach ($idValue in $ids) {
        $process = Get-Process -Id $idValue -ErrorAction SilentlyContinue
        if ($process) {
            $sample[$idValue] = [PSCustomObject]@{
                Cpu = $process.TotalProcessorTime.TotalSeconds
                WorkingSet = $process.WorkingSet64
                PrivateBytes = $process.PrivateMemorySize64
            }
        }
    }
    return $sample
}

$before = Get-BoboSample
$watch = [System.Diagnostics.Stopwatch]::StartNew()
Start-Sleep -Seconds $Seconds
$after = Get-BoboSample
$watch.Stop()
$cpu = 0.0
foreach ($key in $after.Keys) {
    if ($before.ContainsKey($key)) { $cpu += [math]::Max(0, $after[$key].Cpu - $before[$key].Cpu) }
}
$result = [PSCustomObject]@{
    Label = $Label
    SampleSeconds = [math]::Round($watch.Elapsed.TotalSeconds,2)
    ProcessCount = $after.Count
    MainWorkingSetMiB = [math]::Round($after[$BoboProcessId].WorkingSet / 1MB,2)
    MainPrivateBytesMiB = [math]::Round($after[$BoboProcessId].PrivateBytes / 1MB,2)
    MainCpuSeconds = [math]::Round([math]::Max(0,$after[$BoboProcessId].Cpu - $before[$BoboProcessId].Cpu),4)
    TreeWorkingSetMiB = [math]::Round(($after.Values | Measure-Object WorkingSet -Sum).Sum / 1MB,2)
    TreePrivateBytesMiB = [math]::Round(($after.Values | Measure-Object PrivateBytes -Sum).Sum / 1MB,2)
    TreeCpuSeconds = [math]::Round($cpu,4)
}
$result | ConvertTo-Json
