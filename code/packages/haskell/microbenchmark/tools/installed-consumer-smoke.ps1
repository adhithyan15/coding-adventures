$ErrorActionPreference = 'Stop'
$temporary = Join-Path ([System.IO.Path]::GetTempPath()) ("haskell-consumer-{0}" -f [guid]::NewGuid())
New-Item -ItemType Directory -Path $temporary | Out-Null
try {
  $environment = Join-Path $temporary 'consumer.ghc.environment'
  cabal install --lib "--package-env=$environment" ../rank-statistics .
  if ($LASTEXITCODE -ne 0) { throw "Cabal library install exited $LASTEXITCODE" }
  $env:GHC_ENVIRONMENT = $environment
  $objects = Join-Path $temporary 'objects'
  New-Item -ItemType Directory -Path $objects | Out-Null
  ghc -outputdir $objects -o (Join-Path $temporary 'installed-consumer.exe') tools/InstalledConsumer.hs
  if ($LASTEXITCODE -ne 0) { throw "installed consumer compile exited $LASTEXITCODE" }
  & (Join-Path $temporary 'installed-consumer.exe')
  if ($LASTEXITCODE -ne 0) { throw "installed consumer exited $LASTEXITCODE" }
} finally {
  Remove-Item Env:GHC_ENVIRONMENT -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
}
