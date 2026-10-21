---
category: Testing & coverage
---

# A matrix that trims stdout cannot see a leading or trailing empty line, so put print() between other output

**What went wrong.** The first `py-int-print` corpus case for `print()` began
with it: `print()\nprint(1); print(2);\nprint(3,)\n`, expecting
`"\n1\n2\n3\n"`. The corpus test compares stdout exactly, so it would have
caught a missing newline there. But `lang_matrix` compares
`stdout.trim()` against the trimmed expectation, and its rows must be the
corpus cases verbatim. The matrix row for that case would therefore have
passed on a backend whose `putchar 10` wrote nothing at all. The leading
empty line is exactly what the row was meant to prove.

**Fix.** The case became `print(1)\nprint()\nprint(2); print(3);\nprint(4,)\n`,
expecting `1\n\n2\n3\n4`. The empty line now sits between two printed values,
where trimming cannot remove it. Ruby's 5a rows place `puts()` mid-output
for the same reason.

**Next time.** Before adding a matrix row whose evidence is whitespace (an
empty line, a trailing newline, a space separator), check how the runner
compares output. If it trims, put the whitespace between non-whitespace
output, or assert on untrimmed bytes in a dedicated test.
