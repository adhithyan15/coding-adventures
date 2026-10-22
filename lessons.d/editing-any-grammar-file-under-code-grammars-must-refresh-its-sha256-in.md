---
category: CI & GitHub Actions
---

# Editing any grammar file under code/grammars must refresh its sha256 in the language-frontend fixture ledger

`code/specs/fixtures/language-frontends-v1/grammars.json` pins a sha256 of
every grammar file it lists, ecmascript `.tokens` and `.grammar` included.
The "Repo-wide metadata contracts" check (`code/scripts/language_frontend_fixtures.py`)
fails with `ValueError: stale grammar digest` when a pinned file changes. An
existing lesson covers *adding* a grammar file, but *editing* one trips the
same gate. #17901 (tokens) and #17899 (grammars) both hit it.

Do this: after editing any file under `code/grammars/`, recompute the sha256
of each changed file listed in that ledger, then run
`python3 code/scripts/language_frontend_fixtures.py` before pushing. There is
no regeneration command; edit the digest values in place.
