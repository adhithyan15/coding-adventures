set -e

# Step 1: Build the Rust cdylib extension.
# cargo, python3, and uv are in PATH (mise shims locally; GitHub Actions installs on CI).
# All other Python BUILD files in this repo use tools directly — same pattern here.
cd ext/conduit_native && cargo build --release && cd ../..

# Step 2: Copy the built cdylib into the Python package directory with the
# platform-specific Python extension suffix (e.g. .cpython-313-linux-gnu.so).
# NOTE: The build tool runs each BUILD line in its own shell (exec.Command per line),
# so shell variables do NOT persist between lines. We use a Python one-liner to do
# the glob + copy atomically — no multi-line if/fi, no cross-line variable refs.
python3 -c "import sysconfig, glob, shutil, sys; ext = sysconfig.get_config_var('EXT_SUFFIX'); libs = glob.glob('ext/conduit_native/target/release/libconduit_native.*'); src = next((l for l in libs if l.endswith(('.so', '.dylib', '.dll'))), None); sys.exit('ERROR: Rust extension build failed -- no libconduit_native found') if src is None else shutil.copy(src, 'src/coding_adventures/conduit/conduit_native' + ext)"

# Step 3: Create a virtual environment and install the package in editable mode.
uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet

# Step 4: Run the test suite.
.venv/bin/python -m pytest tests/ -v
