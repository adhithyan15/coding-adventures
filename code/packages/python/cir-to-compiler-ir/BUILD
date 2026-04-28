uv venv --quiet --clear
uv pip install -e ../compiler-ir -e ../interpreter-ir -e ../ir-optimizer -e ../codegen-core -e ../wasm-leb128 -e ../wasm-types -e ../wasm-opcodes -e ../wasm-module-parser -e ../wasm-validator -e ../wasm-execution -e ../wasm-runtime -e ../wasm-module-encoder -e ../virtual-machine -e ../ir-to-wasm-compiler -e ../jvm-class-file -e ../ir-to-jvm-class-file -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
