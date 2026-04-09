#!/bin/bash
# Build and test script for the lz77 Python package.

set -euo pipefail

# Create isolated virtual environment.
uv venv --quiet --clear

# Install package with dev dependencies.
uv pip install -e ".[dev]" --quiet

# Run tests with coverage report.
.venv/bin/python -m pytest tests/ -v
