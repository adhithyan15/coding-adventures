$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..\example\consumer')
try {
  mix deps.get --quiet
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  mix run -e "if MicrobenchmarkConsumer.run() != :ok, do: System.halt(1)"
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  Pop-Location
}
