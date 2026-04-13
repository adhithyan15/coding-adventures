if [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ]; then REPO_ROOT=$(git rev-parse --show-toplevel) && (cd "$REPO_ROOT/code/packages/lua/paint_vm_metal_native" && bash BUILD) && (cd "$REPO_ROOT/code/packages/lua/paint_codec_png_native" && bash BUILD); fi
cd tests && busted . --verbose --pattern=test_
