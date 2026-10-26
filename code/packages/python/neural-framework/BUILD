python -m pip install --no-deps -e ../matrix
python -m pip install --no-deps -e .
python -m pip install pytest pytest-cov ruff
ruff check src tests examples
python -m pytest
python examples/xor.py
