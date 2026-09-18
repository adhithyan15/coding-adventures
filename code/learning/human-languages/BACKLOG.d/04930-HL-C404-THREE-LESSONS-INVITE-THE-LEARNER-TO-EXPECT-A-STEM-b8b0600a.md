## HL-C404 — three lessons invite the learner to expect a stem change "before something else", and two of them are probably right

Raised in review of the `HL-C400` fix. **Filed rather than patched**, and filed
with a dissent, because on inspection the reviewer's grouping does not hold.

### The three sites

| lesson | seq | wording |
|---|---|---|
| `ML-C88-compound-numbers` | 3100 | *"**A word wears one shape alone and another before something else**, and this language does it often enough that it is worth expecting rather than learning case by case. You have now met it on a place name and on a number."* |
| `ML-R88-numbers-recall` | 3110 | *"**കേരളം** became **കേരളത്തി**ൽ before its ending; a ten does the same before a digit. **A word wears one shape alone and another before something else** — worth expecting from here on."* |
| `ML-C89-mosham` | 3130 | *"You have seen this book change a word's shape before something else attaches; **this is the same habit in a smaller place**, and here **it is only the writing that moves**."* |

### Why these are not `HL-C400` sites

`HL-C400` was about a specific false claim: that the **ം** is *dropped* whenever
anything attaches. **None of these three says that.** They make a much weaker and
different claim — that Malayalam stems alternate before a following element —
and that claim is **true**:

- **കേരളം** → **കേരളത്തി-** before a case ending (`ML-C83`)
- **ഇരുപത്** → **ഇരുപത്തി-** before a digit (`ML-C88`)

Note that the second has **no ം at all** and takes **no case ending**. That is
the point: the generalisation these lessons make is about *stem alternation*,
which genuinely covers both, and not about the anusvara, which does not.

**`ML-C88` and `ML-R88` therefore look correct as written**, and narrowing them
to "a case ending" would make them **false**, because the ten's alternation is
not triggered by a case ending.

### The one that is arguably borderline

`ML-C89-mosham` calls the **ം** → **മ** writing change *"the same habit"* as the
stem change. `ML-C96-eluppam` (3410) later insists these are different events —
*"Not every ending is this quiet"* — and chapter 97 warns *"do not carry any of
it across to the ം"*.

But `ML-C89` **distinguishes them in its own next clause**: *"and here it is only
the writing that moves."* So it says *same habit*, then immediately says what is
not the same. Whether that is enough is a judgement about how much weight *"the
same habit"* carries before the qualifier lands.

### Why this was not fixed in the `HL-C400` pass

The same pass **introduced** a false claim by over-reaching: narrowing
`ML-C91-vila-panam` to *"a case ending"* turned its loose analogy *"as it did on
the state name and on the tens"* into an assertion that a case ending removed a
**ം** from the tens — which have neither. That was caught in review and fixed.

Widening the same pass into three more lessons whose claim had **not** been
enumerated would have been the same mistake twice in one commit. The honest
move is to record the finding, record the dissent, and let whoever takes it
enumerate first.

### What that enumeration should establish

Whether the corpus's stem alternations form one class the learner can usefully
expect (`ML-C88`'s position) or two that should be kept apart (`ML-C96`'s), and
whether **ം** → **മ** belongs with either. That is a question about how to teach
the system, not about what the system is.
