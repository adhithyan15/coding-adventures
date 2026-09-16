---
category: Testing & coverage
---

# A suite that captures stdout through StringIO cannot see an encoding defect on any platform

`python code/scripts/lessons.py index` crashed partway on a Windows console —
**31 lines of 526**, exit 1 — for as long as the command had existed. Every CI
run was green, and so was every local run of the suite.

`_meta.md` names that command as the way to survey the corpus before adding a
lesson, so on Windows a near-duplicate check silently examined 6% of it and
returned a clean-looking short listing.

### The reason is the capture, not the platform

The tempting explanation is CI topology — the gate runs on Linux, where stdout
is UTF-8, so a Windows-only defect is invisible. I wrote two versions of that
explanation and both were false. The experiment that settles it:

```
pre-fix module + pre-fix suite + the 526-shard corpus
PYTHONIOENCODING=cp1252 python -m unittest discover -s code/scripts/tests \
                                                    -p test_lessons.py
  -> Ran 28 tests ... OK      exit 0
```

Green, against the **unfixed** module, on a cp1252 stream. Adding that suite to
a Windows leg would have caught nothing.

Two measurements explain it. The pre-fix suite called `cmd_index` **zero**
times — so the crashing command was never exercised on any platform. And its 14
command invocations all routed through one helper:

```python
def run(command, *args) -> tuple[int, str, str]:
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = command(*args)
    ...  # returns code and both captured strings
```

**`io.StringIO` accepts every codepoint.** It has no encoding, so it cannot
raise `UnicodeEncodeError`, on Linux, macOS or Windows alike. A suite built on
it is blind to this entire defect class by construction — the platform it runs
on is irrelevant.

### What a test has to touch instead

The regression test runs the real CLI as a **subprocess** with
`PYTHONIOENCODING=cp1252` and asserts one emitted line per loaded lesson. Two
earlier in-process versions reconfigured the replacement stream *inside the test
body*, so they exercised their own fix rather than the module's and passed with
the module fix deleted.

I did not notice by reading them. I noticed by copying the module, stripping the
two-line fix and re-running: both still passed. **A regression test earns its
name only after you have watched it fail.** The cheapest way to watch is to
remove the fix from a copy.

If a test must stay in-process, `io.TextIOWrapper(io.BytesIO(), encoding=...)`
is a real stream with a real codec and does raise. `StringIO` is a convenience
that silently widens what the test accepts.

### Why one stream crashed and the other could not

7 titles carry a character cp1252 cannot encode — `U+2192` in five, `U+2260` in
the other two, over the 526-shard corpus at `83e38b3a3b`. Yet `validate` prints
problem text and never failed:

```
sys.stdout   cp1252 / strict             <- raises
sys.stderr   cp1252 / backslashreplace   <- cannot raise, whatever the payload
```

CPython gives stderr `backslashreplace` on every platform. When a program
half-works, check whether the working half is simply writing to the more
forgiving stream before reaching for a platform explanation.

The `strict` on that first row is worth pinning down, because it is not
unconditional. Measured on the same box: a console, or a pipe with
`PYTHONIOENCODING=cp1252`, gives `cp1252 / strict` and raises; a **bare pipe**
with the variable unset gives `cp1252 / surrogateescape`, which does not. So the
crash needs a console or an explicit encoding — which is exactly the shape a
redirected local run hides, and one more reason the regression test sets
`PYTHONIOENCODING` rather than trusting the ambient locale.

### The two false explanations, kept on purpose

Both were conclusions published ahead of the measurement, and the second failed
the same way as the first:

1. *"every CI runner is Linux"* — false; the repo also uses `windows-latest`,
   `windows-2025`, `macos-latest`, `macos-15`, `macos-15-intel`, `macos-14`.
   The census had been scoped to one file, and a `[a-z0-9.-]*` character class
   silently captured `${{ matrix.os }}` as an empty string.
2. *"`code/scripts/tests` really is run on Windows by `release-engram.yml` and
   `release-venture.yml`"* — also false. A grep showed each **file** contains a
   Windows runner **and** a discover line, and I concluded they co-occur. **File
   level co-occurrence is not job-level co-occurrence**: both files' discover
   sites sit in a `validate` job on `ubuntu-latest`.

The second one argued *against* the thesis it was offered to support — on that
evidence the directory, not the pattern, would have been the pin. A review
caught it, and the experiment above then retired the whole platform framing.
**When the third version of an explanation is still being repaired, the
explanation is usually the wrong shape.**

Related: [[a-zero-occurrence-count-is-a-question-with-several-answers-not-a]] —
a zero may be the instrument;
[[count-what-you-inspected-or-the-loop-that-inspects-nothing-reports-success]] —
a loop whose input was empty reports success;
[[i-built-the-exact-vacuous-check-i-had-spent-the-day-criticising]] — a control
that only tests the axis already in mind.
