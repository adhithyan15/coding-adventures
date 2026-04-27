cpanm --notest --quiet Test2::V0
if [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ]; then (cd ../paint_vm_metal_native && bash BUILD) && (cd ../paint_codec_png_native && bash BUILD); fi
if command -v mise >/dev/null 2>&1; then PROVE_BIN=$(mise which prove); else PROVE_BIN=$(command -v prove); fi; "$PROVE_BIN" -I"$PWD/lib" -I"$(cd ../code39 && pwd)/lib" -I"$(cd ../paint-instructions && pwd)/lib" -I"$(cd ../barcode-layout-1d && pwd)/lib" -I"$(cd ../pixel-container && pwd)/lib" -I"$(cd ../paint_vm_metal_native && pwd)/lib" -I"$(cd ../paint_codec_png_native && pwd)/lib" -v t/
