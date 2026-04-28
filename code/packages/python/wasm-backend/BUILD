uv venv --quiet --clear
uv pip install -e ../compiler-ir -e ../interpreter-ir -e ../ir-optimizer -e ../codegen-core -e ../cir-to-compiler-ir -e ../wasm-leb128 -e ../wasm-types -e ../wasm-opcodes -e ../wasm-module-parser -e ../wasm-validator -e ../wasm-execution -e ../wasm-runtime -e ../wasm-module-encoder -e ../virtual-machine -e ../ir-to-wasm-compiler -e ../simulator-protocol -e ../vm-core -e ../jit-core -e ../tetrad-lexer -e ../tetrad-parser -e ../tetrad-type-checker -e ../tetrad-compiler -e ../tetrad-runtime -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
