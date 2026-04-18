uv venv --quiet --clear
uv pip install pytest pytest-cov ruff --quiet
cargo build --release
if [ -f .venv/Scripts/python.exe ]; then PYTHON_BIN=.venv/Scripts/python; else PYTHON_BIN=.venv/bin/python; fi
BUILD_IMPORT_DIR="$($PYTHON_BIN - <<'PY'
import tempfile
print(tempfile.mkdtemp(prefix="window-native-build-", dir="."))
PY
)"
trap 'rm -rf "$BUILD_IMPORT_DIR"' EXIT
mkdir -p "$BUILD_IMPORT_DIR/window_native"
cp src/window_native/__init__.py "$BUILD_IMPORT_DIR/window_native/__init__.py"
$PYTHON_BIN - "$BUILD_IMPORT_DIR" <<'PY'
from pathlib import Path
import shutil
import sys

build_root = Path(sys.argv[1]) / "window_native"
candidates = [
    (Path("target/release/libwindow_native.so"), "window_native.so"),
    (Path("target/release/libwindow_native.dylib"), "window_native.so"),
    (Path("target/release/window_native.dll"), "window_native.pyd"),
]

for source, name in candidates:
    if source.is_file():
        shutil.copyfile(source, build_root / name)
        break
else:
    raise SystemExit("no compiled extension artifact was found")
PY
PYTHONPATH="$BUILD_IMPORT_DIR" $PYTHON_BIN -m ruff check src tests
PYTHONPATH="$BUILD_IMPORT_DIR" $PYTHON_BIN -m pytest tests/ -v
