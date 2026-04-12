uv venv --quiet --clear
uv pip install pytest --quiet
cargo build --release
if [ -f target/release/libheap_native.so ]; then cp target/release/libheap_native.so src/heap_native/heap_native.so; elif [ -f target/release/libheap_native.dylib ]; then cp target/release/libheap_native.dylib src/heap_native/heap_native.so; elif [ -f target/release/heap_native.dll ]; then cp target/release/heap_native.dll src/heap_native/heap_native.pyd; fi
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src .venv/bin/python -m pytest tests/ -v; fi
