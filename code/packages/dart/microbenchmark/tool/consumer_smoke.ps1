$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..\example\consumer')
try {
  dart pub get
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  dart analyze --fatal-infos
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  dart run bin/consumer.dart
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  Pop-Location
}
