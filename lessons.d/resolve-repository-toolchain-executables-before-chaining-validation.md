# Resolve repository toolchain executables before chaining validation

On a fresh PowerShell process, the Python package validation command assumed
`uv` was still available on `PATH`. The command failed before creating its
virtual environment even though Python 3.13 itself was installed through the
Windows launcher. Before chaining repository validation commands, resolve each
required executable with `Get-Command` (or use the repository's known absolute
tool path). If an optional environment manager is unavailable, use the pinned
interpreter directly to create the virtual environment, then run every check
through that environment's explicit executable path.
