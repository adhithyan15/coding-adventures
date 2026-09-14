# `assert old in s` is not enough for a scripted replace — assert it is UNIQUE (human-language-data)

A Python patch script filled placeholder pins with real numbers. It guarded every
replacement with `assert old in s`, per the existing lesson about scripted edits. It
still corrupted the file: `expect(report.summary.missedByWindow.R2).toBe(0)` appeared
**twice** — once as a corpus pin awaiting its value, once as a legitimate unit-test
assertion that no window is judged on a short track. `str.replace(old, new, 1)` took
the first, which was the unit test.

An earlier no-op guard in the same session produced `expect(report.summary.).toBe(1)`
by replacing a substring with the empty string, which the parser caught only because
it was syntactically invalid.

**Use `assert s.count(old) == 1`, or match on enough surrounding context to be
unique.** Presence proves the target exists; it does not prove you are editing the one
you meant. Tests caught both here — but a non-unique replace that lands on a *valid*
line produces no error at all.
