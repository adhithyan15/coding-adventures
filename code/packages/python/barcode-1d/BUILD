uv venv --quiet --clear
uv pip install --python .venv/bin/python -e ../paint-instructions --quiet
uv pip install --python .venv/bin/python -e ../barcode-layout-1d --quiet
uv pip install --python .venv/bin/python -e ../code39 --quiet
uv pip install --python .venv/bin/python -e ../pixel_container --quiet
if [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ]; then (cd ../paint-vm-metal-native && bash BUILD) && (cd ../paint-codec-png-native && bash BUILD) && uv pip install --python .venv/bin/python -e ../paint-vm-metal-native --quiet && uv pip install --python .venv/bin/python -e ../paint-codec-png-native --quiet; fi
uv pip install --python .venv/bin/python -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
