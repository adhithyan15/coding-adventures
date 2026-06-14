"""Conftest: ensure src layout packages are importable under all test runners.

uv editable installs use .pth files that may not be processed in all Python
versions. This conftest adds all necessary source paths explicitly.
"""
import sys
import os

# Root of the packages directory
_pkg_root = os.path.join(os.path.dirname(__file__), "..")

# Add src directories for all dependencies (read from .pth files)
_site_packages = os.path.join(os.path.dirname(__file__), ".venv", "lib")
# Find the python version dir
for _pyver in os.listdir(_site_packages) if os.path.isdir(_site_packages) else []:
    _sp = os.path.join(_site_packages, _pyver, "site-packages")
    if os.path.isdir(_sp):
        for _f in os.listdir(_sp):
            if _f.startswith("_editable_impl_") and _f.endswith(".pth"):
                _pth = os.path.join(_sp, _f)
                with open(_pth) as _fh:
                    _path = _fh.read().strip()
                if _path and os.path.isdir(_path) and _path not in sys.path:
                    sys.path.insert(0, _path)
