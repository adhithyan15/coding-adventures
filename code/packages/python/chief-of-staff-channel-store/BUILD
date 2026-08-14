uv venv --quiet --clear
uv pip install -e ../md5 -e ../sha1 -e ../sha256 -e ../sha512 -e ../uuid -e ../ed25519 -e ../chacha20-poly1305 -e ../chief-of-staff-channel-crypto -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
.venv/bin/ruff check src tests
.venv/bin/mypy src tests
