PYTHONIOENCODING=utf-8 uv venv .venv --quiet --no-project --clear
uv pip install --python .venv --no-deps -e ../../../packages/python/md5 --quiet
uv pip install --python .venv -e . --quiet
PYTHONIOENCODING=utf-8 uv run --no-project python main.py
