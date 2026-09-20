# Run repository unittest files by discovery or file path, not the code module prefix

Invoking `unittest` with a dotted name beginning in `code.scripts` collided
with Python's standard-library `code` module, so discovery failed before any
repository test ran. Execute these repository tests with `unittest discover`
and a filename pattern from `code/scripts/tests`, or invoke the test file path
directly. Do not assume a directory named `code` is an importable package.
