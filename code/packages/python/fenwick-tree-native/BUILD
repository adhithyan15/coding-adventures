uv venv --quiet --clear
uv pip install pytest --quiet
cargo build --release
if [ -f target/release/libfenwick_tree_native.so ]; then cp target/release/libfenwick_tree_native.so src/fenwick_tree_native/fenwick_tree_native.so; elif [ -f target/release/libfenwick_tree_native.dylib ]; then cp target/release/libfenwick_tree_native.dylib src/fenwick_tree_native/fenwick_tree_native.so; elif [ -f target/release/fenwick_tree_native.dll ]; then cp target/release/fenwick_tree_native.dll src/fenwick_tree_native/fenwick_tree_native.pyd; fi
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src .venv/bin/python -m pytest tests/ -v; fi
