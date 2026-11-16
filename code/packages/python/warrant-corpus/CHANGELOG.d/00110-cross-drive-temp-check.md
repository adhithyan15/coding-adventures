## Unreleased

- `verify_isolated_cache` no longer crashes on Windows when the cache prefix is on a different drive from the temporary directory. `os.path.commonpath` raises `ValueError` there; the new `_is_under` helper treats that as "not under", so the module's own `pycache-isolation` refusal ("it is not inside the private temporary directory") is raised (WD-004b2). Tests reproduce it by monkeypatching `commonpath` and by driving `ntpath.commonpath` directly. The runner source digest moved, so the Python `ct-compare` manifest was regenerated.
