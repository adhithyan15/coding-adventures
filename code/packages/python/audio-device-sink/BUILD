uv venv --quiet --clear
uv pip install pytest pytest-cov ruff --quiet
uv pip install --no-deps -e ../trig --quiet
uv pip install --no-deps -e ../oscillator --quiet
uv pip install --no-deps -e ../pcm-audio --quiet
cargo build --release
BUILD_IMPORT_DIR=$(.venv/bin/python -c 'import tempfile; print(tempfile.mkdtemp(prefix="audio-device-sink-build-", dir="."))') && mkdir -p "$BUILD_IMPORT_DIR/audio_device_sink" && cp src/audio_device_sink/__init__.py "$BUILD_IMPORT_DIR/audio_device_sink/__init__.py" && FOUND_ARTIFACT= && for ARTIFACT in target/release/libaudio_device_sink.so target/release/libaudio_device_sink.dylib target/release/audio_device_sink.dll; do if [ -f "$ARTIFACT" ]; then case "$ARTIFACT" in *.dll) ARTIFACT_NAME=audio_device_sink.pyd ;; *) ARTIFACT_NAME=audio_device_sink.so ;; esac; cp "$ARTIFACT" "$BUILD_IMPORT_DIR/audio_device_sink/$ARTIFACT_NAME"; FOUND_ARTIFACT=1; break; fi; done && if [ -z "$FOUND_ARTIFACT" ]; then echo "no compiled extension artifact was found" >&2; rm -rf "$BUILD_IMPORT_DIR"; exit 1; fi
BUILD_IMPORT_DIR=$(find . -maxdepth 1 -type d -name 'audio-device-sink-build-*' | head -n 1) && PYTHONPATH="$BUILD_IMPORT_DIR" .venv/bin/python -m ruff check src tests
BUILD_IMPORT_DIR=$(find . -maxdepth 1 -type d -name 'audio-device-sink-build-*' | head -n 1) && PYTHONPATH="$BUILD_IMPORT_DIR" .venv/bin/python -m pytest tests/ -v; STATUS=$?; BUILD_IMPORT_DIR=$(find . -maxdepth 1 -type d -name 'audio-device-sink-build-*' | head -n 1); rm -rf "$BUILD_IMPORT_DIR"; exit $STATUS
