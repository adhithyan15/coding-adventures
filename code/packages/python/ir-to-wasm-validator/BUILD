uv venv --quiet --clear
uv pip install -e ../compiler-ir -e ../virtual-machine -e ../wasm-leb128 -e ../wasm-types -e ../wasm-opcodes -e ../wasm-module-parser -e ../wasm-validator -e ../wasm-execution -e ../wasm-runtime -e ../wasm-module-encoder -e ../ir-to-wasm-compiler -e .[dev] --quiet
.venv/bin/python -m pytest tests/ -v
