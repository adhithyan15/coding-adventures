uv venv --quiet --clear
uv pip install pytest pytest-cov --quiet
cargo build --release
if [ -f target/release/libgraph_native.so ]; then cp target/release/libgraph_native.so src/graph_native/graph_native.so; elif [ -f target/release/libgraph_native.dylib ]; then cp target/release/libgraph_native.dylib src/graph_native/graph_native.so; elif [ -f target/release/graph_native.dll ]; then cp target/release/graph_native.dll src/graph_native/graph_native.pyd; fi
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src .venv/bin/python -m pytest tests/ -v; fi
