uv venv --quiet --clear
uv pip install pytest --quiet
cargo build --release
if [ -f target/release/libtree_set_native.so ]; then cp target/release/libtree_set_native.so src/tree_set_native/tree_set_native.so; elif [ -f target/release/libtree_set_native.dylib ]; then cp target/release/libtree_set_native.dylib src/tree_set_native/tree_set_native.so; elif [ -f target/release/tree_set_native.dll ]; then cp target/release/tree_set_native.dll src/tree_set_native/tree_set_native.pyd; fi
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src .venv/bin/python -m pytest tests/ -v; fi
