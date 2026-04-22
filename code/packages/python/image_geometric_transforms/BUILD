uv venv --quiet --clear
uv pip install -e ../pixel_container --quiet
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --cov=image_geometric_transforms --cov-report=term-missing --cov-fail-under=80
