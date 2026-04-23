uv venv --quiet --clear
uv pip install -e ../symbol-core -e ../logic-core -e ../logic-engine -e .\[dev\] --quiet
.venv/bin/python -c "import pathlib, platform; [p.unlink() for p in pathlib.Path('.venv/lib').glob('python*/site-packages/coverage/tracer*.so')] if platform.system() == 'Darwin' else None"
.venv/bin/python -m pytest tests/ -v
