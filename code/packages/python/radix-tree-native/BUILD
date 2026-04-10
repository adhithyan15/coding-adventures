uv venv --quiet --clear
uv pip install pytest --quiet
cargo build --release
if [ -f target/release/libradix_tree_native.so ]; then cp target/release/libradix_tree_native.so src/radix_tree_native/radix_tree_native.so; elif [ -f target/release/libradix_tree_native.dylib ]; then cp target/release/libradix_tree_native.dylib src/radix_tree_native/radix_tree_native.so; elif [ -f target/release/radix_tree_native.dll ]; then cp target/release/radix_tree_native.dll src/radix_tree_native/radix_tree_native.pyd; fi
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src .venv/bin/python -m pytest tests/ -v; fi
