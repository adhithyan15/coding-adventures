---
category: Testing & coverage
---

# A pgrep wait loop whose pattern appears in its own command line never exits, because it matches itself

Waiting for a long `cargo test` to finish, I wrote what looks like the obvious
poll:

```bash
until ! pgrep -f "cargo test --no-fail-fast" >/dev/null; do sleep 15; done
echo "done"; grep -c 'test result: ok' run.log
```

It never printed. The suite had finished minutes earlier — 175 targets, all
green — and I sat through several more polling cycles convinced it was still
building, because every check came back "RUNNING".

`pgrep -f` matches against the **full command line of every process**,
including the shell running the loop. That shell's command line literally
contains the string `cargo test --no-fail-fast`, because the pattern is written
there. So the loop matches itself, the condition is true forever, and the
"wait" is a spin that outlives what it was waiting for. Worse, a second waiter
started later keeps the first one's condition true even after the first exits.

The tell is that the *reported numbers stop moving* while the status stays
"running". A real build advances; a self-match is frozen. I ignored that for
three cycles.

**How to check.** `pgrep -af <pattern>` prints the matching command lines. If
one of them is your own `bash -c … until ! pgrep …`, that is the bug, and it is
obvious the moment you look. Confirm with a name-anchored count:

```bash
pgrep -c -x cargo    # 0 means no cargo process, whatever pgrep -f says
```

**What to do instead.**

* **Don't poll for a command you started.** Start it with the harness's own
  backgrounding and let the completion notification arrive. That is what it is
  for, and it cannot self-match.
* **Match on the program, not the command line.** `pgrep -x cargo` matches the
  executable name and never sees your loop.
* **Wait on the PID you actually launched.** `cmd & pid=$!; wait "$pid"` is
  exact, and immune to any other process's spelling.
* **Wait on the artifact, not the process.** Have the job write a sentinel on
  exit (`cargo test …; echo $? > done.rc`) and poll for that file. This also
  survives the process being reaped by something else.
* If you must use `pgrep -f`, break the self-match with a character class so
  the pattern does not equal the text in your own argv: `pgrep -f "[c]argo
  test"`. The classic `ps | grep "[x]yz"` trick, same reason.

**Related.** The same self-match bites `ps aux | grep foo` (you see the grep),
and `pkill -f` is the dangerous version — a pattern that matches your own shell
will kill the script mid-run. Mine exited 144 doing exactly that.
