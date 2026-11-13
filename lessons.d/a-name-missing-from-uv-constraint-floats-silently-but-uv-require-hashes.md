---
category: Python
---

# A name missing from UV_CONSTRAINT floats silently, but UV_REQUIRE_HASHES rejects editables

A constraints file only restricts packages that are already being resolved; a name it does not list resolves to the newest release with no warning. So pinning is only as good as the file's completeness, and `python3 code/scripts/python_pins.py check` enforces that every third-party name in any pyproject.toml or BUILD/workflow tool install has a pin. The obvious fix for "make unlisted names fail", `UV_REQUIRE_HASHES=1`, cannot be used: it rejects every editable install (`-e ../sibling`), which is how this monorepo links its own packages. Hashes listed in a constraints file are still verified without it. Also: plain `pip install` ignores UV_CONSTRAINT entirely, so BUILD files must use `uv pip install`.
