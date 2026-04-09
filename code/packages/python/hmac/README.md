# coding-adventures-hmac

HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1 — implemented from scratch in Python with no external dependencies beyond the other coding-adventures hash packages.

## Usage

```python
from coding_adventures_hmac import hmac_sha256, hmac_sha256_hex

tag = hmac_sha256(b"secret-key", b"message to authenticate")
print(tag.hex())  # 32-byte / 64-char hex tag

# Or get hex directly
print(hmac_sha256_hex(b"secret-key", b"message"))

# All variants
from coding_adventures_hmac import hmac_md5, hmac_sha1, hmac_sha512
```

## Running Tests

```sh
uv venv && uv pip install -e ../md5 -e ../sha1 -e ../sha256 -e ../sha512 -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```
