set -e

# Step 1: Build the Rust cdylib extension.
# The extension is at ext/conduit_native/ and produces libconduit_native.{so,dylib,dll}.
# We build in release mode for production performance.
cd ext/conduit_native && mise exec -- cargo build --release && cd ../..

# Step 2: Copy the built cdylib into the Python package directory so that
# `from . import conduit_native` works at import time.
#
# Python expects the extension module file to be named:
#   conduit_native<EXT_SUFFIX>
# where EXT_SUFFIX is the platform tag from sysconfig (e.g.
# ".cpython-312-darwin.so" on macOS with Python 3.12).
#
# We create a symlink or copy matching that name so both editable installs
# (uv pip install -e) and direct imports find the extension.
EXT_SUFFIX=$(mise exec -- python3 -c "import sysconfig; print(sysconfig.get_config_var('EXT_SUFFIX'))")
DYLIB=$(ls ext/conduit_native/target/release/libconduit_native.{so,dylib,dll} 2>/dev/null | head -1)
if [ -z "$DYLIB" ]; then
    echo "ERROR: Rust extension build failed — no libconduit_native found" >&2
    exit 1
fi
cp "$DYLIB" "src/coding_adventures/conduit/conduit_native${EXT_SUFFIX}"

# Step 3: Create a Python virtual environment and install the package in
# editable mode (with test dependencies) so pytest can import it.
# Use the mise-managed Python explicitly so the venv matches the .so ABI.
mise exec -- uv venv --python "$(mise exec -- which python3)" --quiet --clear
mise exec -- uv pip install -e ".[dev]" --quiet

# Step 4: Run the test suite.
mise exec -- .venv/bin/python -m pytest tests/ -v
