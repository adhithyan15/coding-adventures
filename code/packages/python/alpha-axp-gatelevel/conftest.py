"""Conftest: ensure src layout packages are importable under all test runners.

uv editable installs use .pth files that may not be processed in all Python
versions (on macOS, uv marks these files with UF_HIDDEN which Python 3.13+
refuses to process). This conftest adds all necessary source paths explicitly.
"""

from __future__ import annotations

import os
import sys

# Walk the site-packages directory and manually process any _editable_impl_*.pth
# files that point to src directories.
_site_packages = os.path.join(os.path.dirname(__file__), ".venv", "lib")
if os.path.isdir(_site_packages):
    for _pyver in os.listdir(_site_packages):
        _sp = os.path.join(_site_packages, _pyver, "site-packages")
        if not os.path.isdir(_sp):
            continue
        for _f in os.listdir(_sp):
            if _f.startswith("_editable_impl_") and _f.endswith(".pth"):
                _pth = os.path.join(_sp, _f)
                try:
                    with open(_pth) as _fh:
                        _path = _fh.read().strip()
                    if _path and os.path.isdir(_path) and _path not in sys.path:
                        sys.path.insert(0, _path)
                except OSError:
                    pass
