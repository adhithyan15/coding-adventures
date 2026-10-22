---
category: Testing & coverage
---

# libtest prints a child test's output on the same line as its test name, so match a marker anywhere in the line

**What went wrong.** LANG84 phase 4b added hostile-input probes that run in
a child copy of the test binary (`current_exe() <child test> --exact
--ignored --nocapture`). The child prints a completion marker, and the parent
looks for it. The parent first matched with `line.strip_prefix(MARKER)`, and
it failed although every probe had passed. With `--nocapture`, libtest writes
`test tests::hostile_probes::child_runs_every_probe ... ` and then the test's
own `println!`, so the marker sat in the middle of a line:

```text
test tests::hostile_probes::child_runs_every_probe ... perl-iir hostile probes passed: 77
ok
```

**Fix.** Use `line.split_once(MARKER)` and take what follows.

**Next time.** When a parent parses a child test's stdout, either match the
marker anywhere in a line, or write it to a channel libtest does not
decorate, such as a file named in an environment variable. Before trusting
the parser, run the child by hand once and look at the raw stdout
(`cat -A`).
