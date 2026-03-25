uv venv --quiet --clear
uv pip install pytest --quiet
cargo build --release
if [ -f target/release/libcommonmark_native.so ]; then cp target/release/libcommonmark_native.so src/commonmark_native/commonmark_native.so; elif [ -f target/release/libcommonmark_native.dylib ]; then cp target/release/libcommonmark_native.dylib src/commonmark_native/commonmark_native.so; elif [ -f target/release/commonmark_native.dll ]; then cp target/release/commonmark_native.dll src/commonmark_native/commonmark_native.pyd; fi
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src .venv/bin/python -m pytest tests/ -v; fi
