uv venv --quiet --clear
uv pip install --no-deps -e ../note-frequency --quiet
uv pip install --no-deps -e ../trig --quiet
uv pip install --no-deps -e ../oscillator --quiet
uv pip install --no-deps -e ../pcm-audio --quiet
uv pip install --no-deps -e ../virtual-dac --quiet
uv pip install --no-deps -e ../virtual-speaker --quiet
uv pip install --no-deps -e ../wav-sink --quiet
uv pip install --no-deps -e ../note-audio --quiet
uv pip install --no-deps -e ../musical-instruments --quiet
uv pip install --no-deps -e . --quiet
uv pip install pytest pytest-cov ruff --quiet
uv run --no-project ruff check src/ tests/
uv run --no-project python -m pytest tests/ -v
