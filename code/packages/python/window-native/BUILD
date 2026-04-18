uv venv --quiet --clear
uv pip install pytest pytest-cov ruff --quiet
cargo build --release
if [ -f .venv/Scripts/python.exe ]; then PYTHON_BIN=.venv/Scripts/python; else PYTHON_BIN=.venv/bin/python; fi
$PYTHON_BIN - <<'PY'
from pathlib import Path
import shutil

candidates = [
    (Path("target/release/libwindow_native.so"), Path("src/window_native/window_native.so")),
    (Path("target/release/libwindow_native.dylib"), Path("src/window_native/window_native.so")),
    (Path("target/release/window_native.dll"), Path("src/window_native/window_native.pyd")),
]

for source, destination in candidates:
    if not source.is_file():
        continue

    for candidate in (Path("src"), destination.parent, destination):
        if candidate.exists() and candidate.is_symlink():
            raise SystemExit(f"refusing to copy build artifact through symlinked path: {candidate}")

    if destination.exists() and not destination.is_file():
        raise SystemExit(f"refusing to overwrite non-file destination: {destination}")

    shutil.copyfile(source, destination)
    break
PY
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m ruff check src tests && PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src .venv/bin/python -m ruff check src tests && PYTHONPATH=src .venv/bin/python -m pytest tests/ -v; fi
