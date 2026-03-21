uv venv --quiet --clear
uv pip install pytest --quiet
cargo build --release
if [ -f target/release/libdirected_graph_native.so ]; then cp target/release/libdirected_graph_native.so src/directed_graph_native/directed_graph_native.so; elif [ -f target/release/libdirected_graph_native.dylib ]; then cp target/release/libdirected_graph_native.dylib src/directed_graph_native/directed_graph_native.so; elif [ -f target/release/directed_graph_native.dll ]; then cp target/release/directed_graph_native.dll src/directed_graph_native/directed_graph_native.pyd; fi
.venv/bin/python -m pytest tests/ -v
