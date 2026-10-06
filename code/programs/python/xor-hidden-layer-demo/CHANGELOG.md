# Changelog

## Unreleased

- **Windows build.** New `BUILD_windows`: `PYTHONPATH=a:b` assignments inside `(cd ... && ...)`
  cannot run under `cmd /C`, and `:` is not Windows' path separator. The Windows file uses
  `set "PYTHONPATH=a;b" &&` and `uv run --no-project --with pytest`, as the repo's other Python
  programs do on Windows.
