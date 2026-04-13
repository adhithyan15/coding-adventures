if command -v mise >/dev/null 2>&1; then
  PERL_BIN=$(mise which perl)
  PROVE_BIN=$(mise which prove)
else
  PERL_BIN=$(command -v perl)
  PROVE_BIN=$(command -v prove)
fi

if [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ]; then
  cargo build --release &&
  soext=$("$PERL_BIN" -MConfig -e 'print $Config{so}') &&
  dlext=$("$PERL_BIN" -MConfig -e 'print $Config{dlext}') &&
  mkdir -p lib/auto/CodingAdventures/PaintVmMetalNative &&
  cp target/release/libPaintVmMetalNative.$soext lib/auto/CodingAdventures/PaintVmMetalNative/PaintVmMetalNative.$dlext &&
  "$PROVE_BIN" -I"$PWD/lib" -I"$PWD/../paint-instructions/lib" -I"$PWD/../pixel-container/lib" -v t/
else
  echo Skipping perl/paint_vm_metal_native on non-macOS-arm64
fi
