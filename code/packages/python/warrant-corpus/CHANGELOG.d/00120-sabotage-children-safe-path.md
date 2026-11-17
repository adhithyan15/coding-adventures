## Unreleased

- `sabotage` children now run as `python -P -m coding_adventures_warrant_corpus` (WD-004b3, VER04i 8.1): the working directory is never put on `sys.path`, so a planted stdlib-named file there cannot shadow the standard library. The runner's own directory still reaches the child through `PYTHONPATH`.
