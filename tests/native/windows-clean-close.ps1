param(
    [string]$Workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

$workspacePath = [IO.Path]::GetFullPath($Workspace)
$application = Join-Path $workspacePath 'src-tauri/target/debug/brickpress.exe'
$testRoot = [IO.Path]::GetFullPath((Join-Path $workspacePath 'artifacts/native'))
$data = [IO.Path]::GetFullPath((Join-Path $testRoot ('clean-close-' + [guid]::NewGuid())))
if (-not $data.StartsWith($testRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'The isolated test data must stay inside the workspace native artifacts directory.'
}
[void][IO.Directory]::CreateDirectory($data)

$previousData = $env:BRICKPRESS_TEST_DATA_DIR
$previousPort = $env:TAURI_WEBDRIVER_PORT
$appProcess = $null
try {
    $env:BRICKPRESS_TEST_DATA_DIR = $data
    $env:TAURI_WEBDRIVER_PORT = '4447'
    $appProcess = Start-Process -FilePath $application -WindowStyle Hidden -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    $window = $null
    while ([DateTime]::UtcNow -lt $deadline) {
        $appProcess.Refresh()
        if ($appProcess.HasExited) { throw 'The test application exited during startup.' }
        if ($appProcess.MainWindowHandle -ne [IntPtr]::Zero -and
            $appProcess.MainWindowTitle -like '* — Brickpress' -and
            -not $appProcess.MainWindowTitle.StartsWith('●')) {
            $window = [Windows.Automation.AutomationElement]::FromHandle($appProcess.MainWindowHandle)
            break
        }
        Start-Sleep -Milliseconds 100
    }
    if ($null -eq $window) { throw 'A clean, initialized native window did not appear.' }
    $pattern = $window.GetCurrentPattern([Windows.Automation.WindowPattern]::Pattern)
    $pattern.Close()
    if (-not $appProcess.WaitForExit(30000)) { throw 'The clean window did not exit after close.' }
    if ($appProcess.ExitCode -ne 0) { throw "The native application exited with code $($appProcess.ExitCode)." }
    $recovery = Get-Content -LiteralPath (Join-Path $data 'recovery.json') -Raw | ConvertFrom-Json
    if ($recovery.version -ne 1) { throw 'Document recovery was not flushed before exit.' }
    Write-Output 'Native clean close exited successfully and flushed version-1 recovery in isolated storage.'
}
finally {
    if ($null -ne $appProcess -and -not $appProcess.HasExited) {
        Stop-Process -Id $appProcess.Id -Force
    }
    $env:BRICKPRESS_TEST_DATA_DIR = $previousData
    $env:TAURI_WEBDRIVER_PORT = $previousPort
}
