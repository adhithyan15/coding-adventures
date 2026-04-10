uv venv --quiet --clear
uv pip install pytest --quiet
cargo build --release
if [ -f target/release/libtrie_native.so ]; then cp target/release/libtrie_native.so src/trie_native/trie_native.so; elif [ -f target/release/libtrie_native.dylib ]; then cp target/release/libtrie_native.dylib src/trie_native/trie_native.so; elif [ -f target/release/trie_native.dll ]; then cp target/release/trie_native.dll src/trie_native/trie_native.pyd; fi
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src .venv/bin/python -m pytest tests/ -v; fi
