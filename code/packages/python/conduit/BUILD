set -e

# Step 1: Build the Rust cdylib extension.
# cargo, python3, and uv are in PATH (mise shims locally; GitHub Actions installs on CI).
# All other Python BUILD files in this repo use tools directly — same pattern here.
cd ext/conduit_native && cargo build --release && cd ../..

# Step 2: Copy the built cdylib into the Python package directory with the
# platform-specific Python extension suffix (e.g. .cpython-313-linux-gnu.so).
EXT_SUFFIX=$(python3 -c "import sysconfig; print(sysconfig.get_config_var('EXT_SUFFIX'))")
DYLIB=$(ls ext/conduit_native/target/release/libconduit_native.{so,dylib,dll} 2>/dev/null | head -1)
if [ -z "$DYLIB" ]; then
    echo "ERROR: Rust extension build failed — no libconduit_native found" >&2
    exit 1
fi
cp "$DYLIB" "src/coding_adventures/conduit/conduit_native${EXT_SUFFIX}"

# Step 3: Create a virtual environment and install the package in editable mode.
uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet

# Step 4: Run the test suite.
.venv/bin/python -m pytest tests/ -v
