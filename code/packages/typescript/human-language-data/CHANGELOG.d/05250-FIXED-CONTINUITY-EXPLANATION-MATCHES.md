### Fixed — continuity distinguishes lexical use from word-history explanation

The forward-language detector now keeps Unicode whole-word matching while
excluding two non-lexical teaching contexts: typed etymology blocks, whose
historical forms belong to the root ledger, and explicit `part + part → whole`
morphology equations, whose pieces are not free-standing word uses. This removes
the known Malayalam `അതെ` / `അത്` and Tamil `புரிகிறது` / `அது` false positives
without a language-specific denylist. Punctuation-adjacent uses in ordinary
learner-facing blocks still report.

The full 23-track remeasurement moved only tracks whose old findings occurred
exclusively in those explanatory contexts:

| Track | Before | After | Removed |
|---|---:|---:|---:|
| Bengali | 6 | 3 | 3 |
| French | 52 | 51 | 1 |
| German | 45 | 43 | 2 |
| Italian | 34 | 32 | 2 |
| Kannada | 15 | 13 | 2 |
| Latin | 25 | 16 | 9 |
| Malayalam | 17 | 12 | 5 |
| Marathi | 5 | 4 | 1 |
| Persian | 9 | 4 | 5 |
| Portuguese | 62 | 59 | 3 |
| Sanskrit | 4 | 3 | 1 |
| Spanish | 403 | 341 | 62 |
| Tamil | 13 | 7 | 6 |
| Telugu | 12 | 10 | 2 |

Arabic, Chinese, Gujarati, Hindi, Japanese, Marwadi, Punjabi, Russian, and Urdu
did not move. Corpus debt falls from 740 to 636 findings; no budget or ratchet is
relaxed, and all remaining references retain their previous ceiling or a tighter
generated snapshot.
