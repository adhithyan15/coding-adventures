if [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ]; then (cd ../paint_vm_metal_native && bash BUILD) && (cd ../paint_codec_png_native && bash BUILD); fi
if command -v mise >/dev/null 2>&1; then
  PROVE_BIN=$(mise which prove)
else
  PROVE_BIN=$(command -v prove)
fi

"$PROVE_BIN" -I"$PWD/lib" -I"$PWD/../code39/lib" -I"$PWD/../paint-instructions/lib" -I"$PWD/../barcode-layout-1d/lib" -I"$PWD/../pixel-container/lib" -I"$PWD/../paint_vm_metal_native/lib" -I"$PWD/../paint_codec_png_native/lib" -v t/
