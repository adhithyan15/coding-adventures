set -e

# Step 1: Build the Rust cdylib extension (all IRC + TCP logic lives in Rust).
# cargo, python3, and uv are in PATH (mise shims locally; GitHub Actions on CI).
cd ext/irc_server_native && cargo build --release && cd ../..

# Step 2: Copy the built cdylib into the Python package directory under the
# platform-specific extension suffix (e.g. .cpython-313-darwin.so).  Each BUILD
# line runs in its own shell, so a Python one-liner does the glob + copy atomically.
python3 -c "import sysconfig, glob, shutil, sys; ext = sysconfig.get_config_var('EXT_SUFFIX'); libs = glob.glob('ext/irc_server_native/target/release/libirc_server_native.*'); src = next((l for l in libs if l.endswith(('.so', '.dylib', '.dll'))), None); sys.exit('ERROR: Rust extension build failed -- no libirc_server_native found') if src is None else shutil.copy(src, 'src/coding_adventures/irc_server_native/irc_server_native' + ext)"

# Step 3: Create a virtual environment and install the package in editable mode.
uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet

# Step 4: Run the test suite.
.venv/bin/python -m pytest tests/ -v
