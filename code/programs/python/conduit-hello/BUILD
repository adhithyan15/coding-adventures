set -e
# conduit-hello is a runnable program, not an installable package.
# It depends on coding-adventures-conduit (the Python conduit package).
# We build the conduit package first (which compiles the Rust extension),
# then install conduit-hello's test dependencies and run the tests.

# Step 1: Build the conduit package (Rust extension + Python install).
# This assumes we are already in the conduit-hello directory, so we
# navigate to the package directory relative to this BUILD file.
(cd ../../packages/python/conduit && bash BUILD)

# Step 2: Install test dependencies for conduit-hello using the same venv.
mise exec -- uv pip install pytest pytest-cov --quiet

# Step 3: Run the conduit-hello tests.
mise exec -- ../../packages/python/conduit/.venv/bin/python -m pytest tests/ -v
