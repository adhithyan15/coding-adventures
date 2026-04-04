uv venv --quiet --clear
uv pip install pytest --quiet
cargo build --release
if [ -f target/release/libgf256_native.so ]; then cp target/release/libgf256_native.so src/gf256_native/gf256_native.so; elif [ -f target/release/libgf256_native.dylib ]; then cp target/release/libgf256_native.dylib src/gf256_native/gf256_native.so; elif [ -f target/release/gf256_native.dll ]; then cp target/release/gf256_native.dll src/gf256_native/gf256_native.pyd; fi
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src .venv/bin/python -m pytest tests/ -v; fi
