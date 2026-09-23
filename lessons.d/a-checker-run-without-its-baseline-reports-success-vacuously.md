---
category: CI & GitHub Actions
---

# A checker run without its baseline reports success vacuously

The human-language books workflow scans every compiled `book.log` and fails a
track whose LaTeX warning counts EXCEED the numbers in
`core/latex-warning-baseline.json`. Before pushing a Spanish vocabulary tranche
I ran the same script locally:

```sh
python3 code/scripts/scan_latex_log_warnings.py --book-root code/learning/human-languages
```

It printed `spanish: overfull=0 underfull=1 ... [unseeded]` and exited 0. I read
the two counters I was looking for, saw `overfull=0` and `missing_character=0`,
and pushed. CI then failed:

```
spanish underfull rose to 1 against a baseline of 0
```

**The regression was in my own output.** The script had measured it and printed
it. What it could not do was call it a regression, because I had omitted
`--baseline`, so it had nothing to compare against — and it says so, in the
`[unseeded]` tag at the end of every line. An exit code of 0 from that run means
"nothing was checked", not "nothing is wrong".

**The general shape.** A comparison tool given no comparison input degrades to a
reporting tool, and a reporting tool always succeeds. The failure mode is worse
than a crash would be: it produces a full, correct-looking report and a zero
exit status, so it reads as evidence. Baselines, golden files, snapshot
fixtures, coverage thresholds and diff bases all behave this way.

**What to do.** Run a gate the way CI runs it, flags included — read the
workflow step and copy the whole command rather than the script name. If a run
prints a per-item status word (`[ok]`, `[unseeded]`, `[skipped]`, `[no
baseline]`), that word is the first thing to read, before any number the tool
reports. And when reporting verification afterwards, quote the status, not just
the counters: "spanish `[ok]` against baseline" is a claim, where "overfull=0"
on its own is a measurement that may be compared to nothing.
