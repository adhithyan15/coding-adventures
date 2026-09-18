### Fixed — `HL-C402`: the genitive table claimed to be exhaustive, and the fix needed no new morphology

- No coverage change. `ML-A1-LEX-06` stays closed at 208/243 (86%).

#### Three chapters deferred this, and the blocker dissolved on inspection

`HL-C402` recorded that `ML-C78-ude` (2700), its recall partner
`ML-R78-genitive-recall` (2710) and `ML-C84-mukalil` (2920) all teach *"a noun
that ends in a vowel takes **-യുടെ**"*, with a two-row table:
`a consonant → -ന്റെ` / `a vowel → -യുടെ`.

The shard proposed adding a third row — **പശു** → **പശുവിന്റെ** — and flagged
that row as *the one claim resting on morphology rather than on a grep*. That is
why it sat open across chapters 99, 100 and 101.

**Enumerating every genitive the corpus actually attests dissolved it:**

| stem ends in | genitive | witness | first at |
|---|---|---|---|
| **അ** | **-യുടെ** | **അമ്മയുടെ**, **കസേരയുടെ** | 2700 |
| chillu **ൻ** | **-ന്റെ** | **അധ്യാപകന്റെ** | 2690 |
| half-u | **-ിന്റെ** | **സുഹൃത്തിന്റെ** | **3000** |
| **ം** | **-ത്തിന്റെ** | **പുസ്തകത്തിന്റെ** | **3210** |

**At sequence 2700 the table is accurate for everything the learner has seen** —
only **അധ്യാപകന്റെ** (2690) and **അമ്മയുടെ** exist by then. The falsification
arrives *later*, at 3000 and 3210, on stems that fit neither column.

So the defect was never a missing row. **It was that the table presented itself
as exhaustive** — and the fix is the same one-clause narrowing the ya-glide
sites got, asserting no unattested form.

#### What changed

- `ML-C78-ude` — the rule now names **അമ്മ** instead of all vowel-final nouns,
  the table rows name their own nouns, and one sentence says it outright: *"Two
  nouns, two landings — and not the whole story… this book will show you another
  before long."*
- `ML-R78-genitive-recall` — its Wrap-up **drilled the universal** (*"Which
  ending follows a vowel?"*). It now asks about **അധ്യാപകൻ** and **അമ്മ** by
  name, and adds *"What decides it? **The noun's last sound** — never you."*
- `ML-C84-mukalil` — its Warm-up said *"the one a word ending in a vowel takes"*;
  now *"the one **അമ്മ** took."*

**The forward promise was checked, not assumed.** `ML-C86-oppam` (3000) and
`ML-C91-vila-panam` (3210) both **explain** their third landing — *"the
owner-form takes the ending consonants take"* and *"**പുസ്തകം** changed shape
before its ending"* — rather than merely using it. So *"will show you another
before long"* is true and verifiable.

**പശുവിന്റെ is asserted nowhere.** The corpus refuted the table with words
already in the book, which is what three chapters of deferral were waiting for
and never needed.

#### `HL-C403` filed

`ML-C83-keralathil` (2900) says *"**Every** noun ending in **-ം** does it"* and
*"the moment **anything** is added"*, of the **-ത്ത-** oblique. Five taught
words put something after a **ം** with no **-ത്ത-** in sight — **മോശമാണ്**,
**എളുപ്പമാണ്**, **സുന്ദരമാണ്**, **നൃത്തമാണ്**, **ഉയരമുണ്ട്** — and
**സുന്ദരവും** goes a third way again.

Unlike `HL-C400`–`402`, **this one's correction already exists in the corpus**:
`ML-C96-eluppam` (3410) states the true version in full. It is only in the wrong
place relative to the claim, which makes it a question about teaching order
rather than about Malayalam — so it is filed rather than patched.
