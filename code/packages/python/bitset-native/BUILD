uv venv .venv --quiet --no-project --no-config
uv pip install --no-config --python .venv pytest --quiet
cargo build --release
if [ -f target/release/libbitset_native.so ]; then cp target/release/libbitset_native.so src/bitset_native/bitset_native.so; elif [ -f target/release/libbitset_native.dylib ]; then cp target/release/libbitset_native.dylib src/bitset_native/bitset_native.so; elif [ -f target/release/bitset_native.dll ]; then cp target/release/bitset_native.dll src/bitset_native/bitset_native.pyd; fi
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src uv run --no-project --no-config python -m pytest tests/ -v; fi
