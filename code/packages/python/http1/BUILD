set -eu

if command -v uv >/dev/null 2>&1
then
  uv venv --quiet --clear
  uv pip install -e ../http-core --quiet
  uv pip install --no-deps -e .\[dev\] --quiet
  uv run --no-project python -m pytest tests/ -v
else
  python3 -m venv .venv
  .venv/bin/python -m pip install --quiet ../http-core
  .venv/bin/python -m pip install --quiet --no-deps .\[dev\]
  .venv/bin/python -m pytest tests/ -v
fi
