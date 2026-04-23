$ErrorActionPreference = "Stop"

$source = Join-Path $PSScriptRoot "..\\test\\fixtures\\square_nostd.rs"
$output = Join-Path $PSScriptRoot "..\\test\\fixtures\\square_nostd.wasm"

rustc `
  --target wasm32-unknown-unknown `
  --crate-type cdylib `
  -O `
  -C panic=abort `
  -C debuginfo=0 `
  $source `
  -o $output

Write-Output "Wrote $output"
