uv venv --quiet --clear
uv pip install pytest --quiet
cargo build --release
if [ -f target/release/libimmutable_list_native.so ]; then cp target/release/libimmutable_list_native.so src/immutable_list_native/immutable_list_native.so; elif [ -f target/release/libimmutable_list_native.dylib ]; then cp target/release/libimmutable_list_native.dylib src/immutable_list_native/immutable_list_native.so; elif [ -f target/release/immutable_list_native.dll ]; then cp target/release/immutable_list_native.dll src/immutable_list_native/immutable_list_native.pyd; fi
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src .venv/bin/python -m pytest tests/ -v; fi
