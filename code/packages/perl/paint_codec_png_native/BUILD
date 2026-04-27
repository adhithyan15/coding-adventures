if command -v mise >/dev/null 2>&1; then
  PERL_BIN=$(mise which perl)
  PROVE_BIN=$(mise which prove)
else
  PERL_BIN=$(command -v perl)
  PROVE_BIN=$(command -v prove)
fi

cargo build --release &&
soext=$("$PERL_BIN" -MConfig -e 'print $Config{so}') &&
dlext=$("$PERL_BIN" -MConfig -e 'print $Config{dlext}') &&
mkdir -p lib/auto/CodingAdventures/PaintCodecPngNative &&
cp target/release/libPaintCodecPngNative.$soext lib/auto/CodingAdventures/PaintCodecPngNative/PaintCodecPngNative.$dlext &&
"$PROVE_BIN" -I"$PWD/lib" -I"$PWD/../pixel-container/lib" -v t/
