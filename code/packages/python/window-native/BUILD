uv venv --quiet --clear
uv pip install pytest pytest-cov ruff --quiet
cargo build --release
if [ -f .venv/Scripts/python.exe ]; then PYTHON_BIN=.venv/Scripts/python; else PYTHON_BIN=.venv/bin/python; fi
BUILD_IMPORT_DIR=$("$PYTHON_BIN" -c 'import tempfile; print(tempfile.mkdtemp(prefix="window-native-build-", dir="."))')
trap 'rm -rf "$BUILD_IMPORT_DIR"' EXIT
mkdir -p "$BUILD_IMPORT_DIR/window_native"
cp src/window_native/__init__.py "$BUILD_IMPORT_DIR/window_native/__init__.py"
FOUND_ARTIFACT=
for ARTIFACT in target/release/libwindow_native.so target/release/libwindow_native.dylib target/release/window_native.dll; do
    if [ -f "$ARTIFACT" ]; then
        case "$ARTIFACT" in
            *.dll) ARTIFACT_NAME=window_native.pyd ;;
            *) ARTIFACT_NAME=window_native.so ;;
        esac
        cp "$ARTIFACT" "$BUILD_IMPORT_DIR/window_native/$ARTIFACT_NAME"
        FOUND_ARTIFACT=1
        break
    fi
done
if [ -z "$FOUND_ARTIFACT" ]; then
    echo "no compiled extension artifact was found" >&2
    exit 1
fi
PYTHONPATH="$BUILD_IMPORT_DIR" $PYTHON_BIN -m ruff check src tests
PYTHONPATH="$BUILD_IMPORT_DIR" $PYTHON_BIN -m pytest tests/ -v
