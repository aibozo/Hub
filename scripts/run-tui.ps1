#!/usr/bin/env pwsh
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Simple launcher: starts assistant-core in background (if not already up),
# waits for readiness, runs the TUI, then stops the core on exit.

function Write-Log($msg) { Write-Host "[tui-run] $msg" }

Push-Location (Join-Path $PSScriptRoot '..')
try {
  $bind = if ($env:FOREMAN_BIND) { $env:FOREMAN_BIND } else { '127.0.0.1:6061' }
  $readyUrl = "http://$bind/ready"

  function Test-Ready {
    try {
      $r = Invoke-WebRequest -UseBasicParsing -TimeoutSec 1 -Uri $readyUrl
      return ($r.StatusCode -eq 200)
    } catch { return $false }
  }

  $ownedCore = $false
  $coreProc = $null

  if (Test-Ready) {
    Write-Log "assistant-core already running at $bind"
  } else {
    New-Item -ItemType Directory -Force -Path 'storage/logs' | Out-Null
    $ts = Get-Date -Format 'yyyyMMdd-HHmmss'
    $logFile = "storage/logs/assistant-core-$ts.log"
    Write-Log "Starting assistant-core at $bind (logs: $logFile)"
    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = 'cargo'
    $startInfo.Arguments = 'run -p assistant-core'
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.Environment['RUST_LOG'] = if ($env:RUST_LOG) { $env:RUST_LOG } else { 'info' }
    $startInfo.Environment['FOREMAN_BIND'] = $bind
    $coreProc = New-Object System.Diagnostics.Process
    $coreProc.StartInfo = $startInfo
    $null = $coreProc.Start()
    $ownedCore = $true
    $stdOutWriter = [System.IO.StreamWriter]::new($logFile, $true)
    $coreProc.BeginOutputReadLine()
    $coreProc.BeginErrorReadLine()
    $coreProc.add_OutputDataReceived({ param($s,$e) if ($e.Data) { $stdOutWriter.WriteLine($e.Data) } })
    $coreProc.add_ErrorDataReceived({ param($s,$e) if ($e.Data) { $stdOutWriter.WriteLine($e.Data) } })

    for ($i=0; $i -lt 600; $i++) {
      if (Test-Ready) { break }
      Start-Sleep -Milliseconds 500
    }
    if (-not (Test-Ready)) { throw "assistant-core did not become ready at $readyUrl" }
  }

  Write-Log 'Launching TUI (ui-tui)'
  & cargo run -p ui-tui --features tui,http
  $tuiExit = $LASTEXITCODE

  if ($ownedCore -and $coreProc -and -not $coreProc.HasExited) {
    try { $coreProc.Kill() } catch { }
  }
  exit $tuiExit
}
finally {
  Pop-Location
}
