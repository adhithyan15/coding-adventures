---
category: Compiler / VM / language pipeline
---

# A start-of-period sort key for reduced-precision dates reorders upstream's own release list

LANG84 allows a reduced-precision date (`1993`, `1993-07`) in a
`releases.csv` and says it "sorts as the start of that period". The phase 2a
checker turned that sentence into its order rule: every row's start-of-period
key must be at least the previous row's. Ruby's and Python's inventories
passed that rule.

Perl's perlhist has `5.000alpha4  1993-???-??` listed between alpha3
(1993-10-10) and alpha6 (1994-03-18). It also has `5.000beta3  1994-Sep-??`
between b2c (09-17) and b3a (09-18), and `5.004_01-t1  1997-???-??` right
after 5.004 (1997-05-15). Under the start-of-period rule, alpha4 has to move
ahead of alpha1, and 5.004_01-t1 ahead of every 1997 release, including the
5.003_9x releases it follows. That contradicts the only source for those
dates.

The fix (LANG84 phase 2b) is to treat a reduced date as the whole interval
it names. The order check passes when some day inside each row's period
keeps the column non-decreasing. The checker walks the rows and keeps
`floor`, the latest start date seen so far. A row fails only if even the end
of its period is before `floor`. The migration placed each reduced-date row
where upstream lists it whenever that position falls inside its period. For
fully dated rows the rule is the same as before.

What to do differently: before turning a sort rule into a validator, try it
on the messiest real data, not only on the data the rule was written for.
