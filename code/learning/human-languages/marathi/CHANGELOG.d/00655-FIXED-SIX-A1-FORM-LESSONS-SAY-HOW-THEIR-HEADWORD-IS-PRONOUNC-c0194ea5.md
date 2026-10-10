## Fixed — six A1 form lessons say how their headword is pronounced

Six chapter 25 form lessons showed a Devanagari headword with no `romanization:`, so
the script-closure report counted them in `headwordsWithoutRomanization` (8 -> 2 for
Marathi) and the book's contents line and PDF bookmark showed the script alone. Each
value copies the spelling the MR-A1F01–A1F03 lessons already own, keeping the
headword's `·` separators, `:` labels and `___` blanks:

- MR-A1F04-first-three-select — *nāv · śahar · bhāṣā*
- MR-A1F05-last-three-select — *āvaḍte pey · āvaḍtī kṛtī · mitrāce nāv*
- MR-A1F06-first-three-supported — *nāv: \_\_\_ · śahar: \_\_\_ · bhāṣā: \_\_\_*
- MR-A1F07-first-three-spelling — *Aruṇ · Puṇe · marāṭhī*
- MR-A1F08-last-three-supported — *āvaḍte pey: \_\_\_ · āvaḍtī kṛtī: \_\_\_ · mitrāce nāv: \_\_\_*
- MR-A1F09-last-three-agreement — *-te · -tī · -ce*

**Deliberately left without one (2).** MR-A1F10-first-three-delayed and
MR-A1F11-last-three-delayed are model-free checkpoints: their headwords are the
completed rows being assessed, and MR-A1F10 tells the learner "There is no
romanization and no completed row to copy". A romanization would print that answer in
Latin beside the lesson in the contents and bookmarks.

All six are script lessons, so closure, the exposure exemption and the script ramp are
unchanged. Chapter 25's book section short titles now pair script and romanization;
its narration JSON carries the new field and its spoken text is unchanged.
