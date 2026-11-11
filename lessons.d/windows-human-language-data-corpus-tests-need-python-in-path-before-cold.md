---
category: Testing & coverage
---

# Windows human-language-data corpus tests need Python in PATH before cold-hook benchmarking

While reproducing #19163 on Windows, I trimmed `PATH` to Node, Git, and system
directories. `modality-manifest.test.ts` then failed before its corpus assertions:
the generated-hash owner reader spawned `python` and received `ENOENT`. The
reported wall time was not evidence about the cold hook. Restore the actual
Python runtime directory to `PATH`, confirm `python` resolves, and reset the
generated lesson cache before recording a cold baseline. Keep the suite's
unchanged timeout; do not mask this setup error by relaxing it.
