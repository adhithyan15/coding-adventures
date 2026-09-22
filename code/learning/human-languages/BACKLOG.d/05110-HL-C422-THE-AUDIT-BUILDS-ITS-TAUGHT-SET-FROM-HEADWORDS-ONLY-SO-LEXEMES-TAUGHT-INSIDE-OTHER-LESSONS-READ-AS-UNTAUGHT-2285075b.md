## HL-C422-2285075b — The audit builds its taught set from headwords only, so lexemes taught inside other lessons read as untaught

**Status: OPEN.** Found while pre-checking chapter 472 against A2 mock 2 item
21, by opening the lesson that a grep said already contained `responder`
rather than trusting the audit's "missing" verdict.

This is the **second** metric defect found in two chapters, and it is distinct
from HL-C421. That one is about which text the `requires` rows cover. This one
is about how the *taught set* is built.

### What the code does

`src/spanish-a1-mock-audit-cli.ts` builds `taught` from one field:

```ts
const headword = clean(lesson.realization.headword ?? "");
for (const rawPart of headword.split(/[/,] |\s+y\s+|\s*[—–]\s*/)) {
  taught.add(part);
  taught.add(part.replace(/^(?:el|la|los|las|un|una)\s+/, ""));
  for (const token of part.split(/\s+/)) taught.add(token);
}
```

It never reads `introduces.knowledge`. A lexeme a lesson genuinely teaches,
but which is not that lesson's headword, is invisible to the audit.

### The verified instance

The row for mock 2 item 21 requires `encuesta, preguntar, usuario, responder,
lectura`, and the audit reports **`responder`** among the missing.

`ES-C40-contestar` introduces **`ES-LEX-RESPONDER-06`**. It is not a passing
mention: the lesson carries a section headed *"Grammar Lens: contestar and
responder"*, derives it (*re-* plus *spondēre*, "to pledge solemnly"),
contrasts the pair in a table — *"responder | weightier — responding **to**
something"* — and points out that the noun is *la respuesta*, from *responder*
rather than from *contestar*.

So the word is taught, with more care than most headwords get, and the audit
cannot see it because the lesson is called *contestar*.

### The mechanism to fix it already exists and is nearly empty

The same file carries a hand-maintained escape hatch:

```ts
const citationFormCredits = [
  "llevar", "andar", "dar", "llover", "amigo", "sol",
  "vivir", "llamar", "llamarse", "año", "mes",
];
```

Eleven entries, for lexemes taught under an inflected or partner headword.
`responder` belongs on it and is not there. Nothing has been auditing that
list against `introduces.knowledge`, so there is no reason to think it is
otherwise complete.

### Why this matters, and why it is NOT fixed here

**Do not teach `responder` again.** HL-C418's rule holds: a second lesson for a
word the corpus already has is duplication the reader meets twice. The right
repair is a credit, or teaching the audit to read `introduces`.

The consequence for chapter 472 is worth stating plainly: its row cannot clear
until `responder` is credited, however many genuinely-absent words the chapter
teaches. That is the honest state, and it should be visible rather than
papered over by writing a duplicate lesson to move a number.

Both available repairs move the headline number, which is the programme's
steering signal, so this wants its own branch and its own review — the same
reasoning as HL-C421:

1. **Add the missing credits.** Cheap, but only as good as whoever maintains
   the list.
2. **Read `introduces.knowledge` into the taught set**, mapping atoms to
   surface forms. Correct, larger, and would make the credit list mostly
   redundant. It will also move the number by an unknown amount, which is the
   point of doing it on its own.

Either way the first step is the same: **audit the existing corpus for lexemes
introduced but never headworded**, and report how many there are. Until that
number exists, nobody knows how much of the measured A2 gap is real.
