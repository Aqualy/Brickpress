param(
  [Parameter(Mandatory = $true)][string]$DialogTitle,
  [Parameter(Mandatory = $true)][string]$ApplicationPath
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$expectedApplication = [IO.Path]::GetFullPath($ApplicationPath)
$deadline = [DateTime]::UtcNow.AddSeconds(30)
do {
  $application = Get-Process brickpress -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $expectedApplication } | Select-Object -First 1
  if ($application) {
    $applicationWindows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children,
      [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ProcessIdProperty, [int]$application.Id))
    $titleCondition = [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty, $DialogTitle)
    $dialog = $null
    foreach ($applicationWindow in $applicationWindows) {
      $dialog = $applicationWindow.FindFirst([System.Windows.Automation.TreeScope]::Subtree, $titleCondition)
      if ($dialog) { break }
    }
    if ($dialog) {
      $buttons = $dialog.FindAll([System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button))
      $cancel = $buttons | Where-Object { $_.Current.AutomationId -eq '2' -or $_.Current.Name -eq 'Cancel' } | Select-Object -First 1
      if ($cancel) {
        ([System.Windows.Automation.InvokePattern]$cancel.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke()
        Write-Output "Observed and cancelled: $DialogTitle"
        exit 0
      }
    }
  }
  Start-Sleep -Milliseconds 100
} while ([DateTime]::UtcNow -lt $deadline)
$windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children,
  [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ProcessIdProperty, [int]$application.Id))
$observed = ($windows | ForEach-Object { $_.Current.Name }) -join ', '
throw "The expected dialog did not appear: $DialogTitle. Observed windows: $observed"
