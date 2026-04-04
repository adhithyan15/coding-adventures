uv venv --quiet --clear
uv pip install pytest --quiet
cargo build --release
if [ -f target/release/libpolynomial_native.so ]; then cp target/release/libpolynomial_native.so src/polynomial_native/polynomial_native.so; elif [ -f target/release/libpolynomial_native.dylib ]; then cp target/release/libpolynomial_native.dylib src/polynomial_native/polynomial_native.so; elif [ -f target/release/polynomial_native.dll ]; then cp target/release/polynomial_native.dll src/polynomial_native/polynomial_native.pyd; fi
if [ -f .venv/Scripts/python.exe ]; then PYTHONPATH=src .venv/Scripts/python -m pytest tests/ -v; else PYTHONPATH=src .venv/bin/python -m pytest tests/ -v; fi
