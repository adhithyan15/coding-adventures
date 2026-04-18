uv venv --quiet --clear
uv pip install pytest pytest-cov ruff --quiet
cargo build --release
if [ -f target/release/libwindow_native.so ]; then cp target/release/libwindow_native.so src/window_native/window_native.so; elif [ -f target/release/libwindow_native.dylib ]; then cp target/release/libwindow_native.dylib src/window_native/window_native.so; elif [ -f target/release/window_native.dll ]; then cp target/release/window_native.dll src/window_native/window_native.pyd; fi
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m ruff check src tests && PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src .venv/bin/python -m ruff check src tests && PYTHONPATH=src .venv/bin/python -m pytest tests/ -v; fi
