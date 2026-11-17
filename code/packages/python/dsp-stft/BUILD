uv venv --quiet --clear
uv pip install --no-deps -e ../dsp-complex --quiet
uv pip install --no-deps -e ../dsp-fft --quiet
uv pip install --no-deps -e ../dsp-dct --quiet
uv pip install --no-deps -e ../dsp-filters --quiet
uv pip install --no-deps -e . --quiet
uv pip install pytest pytest-cov ruff --quiet
uv run --no-project ruff check src/ tests/
uv run --no-project python -m pytest tests/ -v
