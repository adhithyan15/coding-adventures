---
category: CI & GitHub Actions
---

# xvfb-run replaces the command's exit status when its temp-directory cleanup fails

On PR #16255, "Build native payload (Compose Desktop / Linux)" failed its
launch check `test "$status" -eq 124` although the app ran its full ten
seconds: the only error was `xvfb-run: error: problem while cleaning up
temporary directory`. After the command exits, xvfb-run stops Xvfb and deletes
its temp directory; when that fails it exits non-zero instead of passing on the
command's status, so `timeout`'s 124 was lost.

Fix: `code/scripts/run-under-xvfb.sh COMMAND...` runs the command under
`xvfb-run -a`, records the command's own status inside the display session and
exits with it (saying so when xvfb-run disagreed); a display that never started
still fails. Every `xvfb-run -a` in ci.yml and release-task-app.yml goes
through it, and `test_run_under_xvfb.py` pins both the behaviour (with a fake
xvfb-run) and that no workflow calls `xvfb-run -a` directly.

Do differently: when a check asserts a wrapped command's exit status, make
sure the wrapper cannot substitute its own.
