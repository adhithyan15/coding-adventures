---
category: Python
---

# os.path.commonpath raises ValueError on Windows when two paths are on different drives: catch it and treat it as not-under, never let a security check crash

`verify_isolated_cache` in the Python warrant-corpus called `os.path.commonpath([temp, real])` to ask whether the private bytecode directory sat under the temp directory. On the Windows CI runner the checkout is on `D:` and `%TEMP%` is on `C:`, and `commonpath` raises `ValueError: Paths don't have the same drive` (it also raises for an absolute and a relative path mixed). The check crashed instead of refusing, so the test that expects the module's own `pycache-isolation` refusal failed. The fix wraps the call in a helper that returns False on `ValueError`: no common root means not under. Reproduce on Linux by monkeypatching `os.path.commonpath` to raise, and by calling `ntpath.commonpath` directly. Any `commonpath` or `relpath` used as a yes/no containment test on Windows needs the same guard, because a different drive is an answer, not an error.
