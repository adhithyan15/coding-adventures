uv venv --quiet --clear
uv pip install pytest --quiet
cargo build --release
# Copy the built shared library into the Python package directory.
# The library name varies by platform:
#   Linux:  libdirected_graph_native.so  -> directed_graph_native.so
#   macOS:  libdirected_graph_native.dylib -> directed_graph_native.so
#   Windows: directed_graph_native.dll -> directed_graph_native.pyd
if [ -f target/release/libdirected_graph_native.so ]; then
    cp target/release/libdirected_graph_native.so src/directed_graph_native/directed_graph_native.so
elif [ -f target/release/libdirected_graph_native.dylib ]; then
    cp target/release/libdirected_graph_native.dylib src/directed_graph_native/directed_graph_native.so
elif [ -f target/release/directed_graph_native.dll ]; then
    cp target/release/directed_graph_native.dll src/directed_graph_native/directed_graph_native.pyd
fi
.venv/bin/python -m pytest tests/ -v
