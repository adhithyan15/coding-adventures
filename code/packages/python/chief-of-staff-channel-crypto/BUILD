uv venv --quiet --clear
uv pip install -e ../md5 -e ../sha1 -e ../sha256 -e ../sha512 -e ../hmac -e ../hkdf -e ../uuid -e ../ed25519 -e ../x25519 -e ../chacha20-poly1305 -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
