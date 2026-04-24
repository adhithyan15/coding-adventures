set -e
# conduit-hello is a runnable program, not an installable package.
# It depends on coding-adventures-conduit (the Python conduit package).
# We build the conduit package first (which compiles the Rust extension and
# creates the .venv with all dev dependencies including pytest), then run
# the conduit-hello tests using that venv's Python directly.
#
# Path: this BUILD file runs from code/programs/python/conduit-hello/
#   ../../../ → code/
#   ../../../packages/python/conduit → code/packages/python/conduit

# Step 1: Build the conduit package (Rust extension + venv + package tests).
(cd ../../../packages/python/conduit && bash BUILD)

# Step 2: Run the conduit-hello E2E tests using the conduit package's venv.
# pytest and pytest-cov are already installed via the conduit package's
# dev extras (see code/packages/python/conduit/pyproject.toml).
../../../packages/python/conduit/.venv/bin/python -m pytest tests/ -v
