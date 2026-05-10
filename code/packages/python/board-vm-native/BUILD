uv venv --quiet --clear
uv pip install pytest --quiet
cargo build --release
if [ -f target/release/libboard_vm_native.so ]; then cp target/release/libboard_vm_native.so src/board_vm_native/board_vm_native.so; elif [ -f target/release/libboard_vm_native.dylib ]; then cp target/release/libboard_vm_native.dylib src/board_vm_native/board_vm_native.so; elif [ -f target/release/board_vm_native.dll ]; then cp target/release/board_vm_native.dll src/board_vm_native/board_vm_native.pyd; fi
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src .venv/bin/python -m pytest tests/ -v; fi
