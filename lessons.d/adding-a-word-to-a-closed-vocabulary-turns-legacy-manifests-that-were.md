---
category: Security boundaries
---

# Adding a word to a closed vocabulary turns legacy manifests that were skipped as unknown into rejected cross-pairs

`test_repository_required_capabilities_use_only_valid_pairs` skips any
capability whose category or action is not in the taxonomy fixture. It treats
that as legacy vocabulary, inventoried separately. When D21 added `spawn` as a
`thread` action, `typescript/forme-sandbox-core`'s `proc:spawn` was no longer
skipped. It became a known-vocabulary invalid cross-pair and failed the gate.
The manifest had been wrong all along, since `proc:spawn` was never a Spec 13
pair, but nothing checked it until the word became known.

The fix was to migrate it to `proc:exec`, which is what starting a launcher
needs.

**Do instead:** before adding a category or action, search every
`required_capabilities.json` for the new word in *either* field:

```bash
python3 - <<'PY'
import json, pathlib
for p in pathlib.Path("code").glob("**/required_capabilities.json"):
    try: m = json.loads(p.read_text())
    except Exception: continue
    for c in (m.get("capabilities") or []) if isinstance(m, dict) else []:
        if isinstance(c, dict) and "NEWWORD" in (c.get("category"), c.get("action")):
            print(p, c.get("category"), c.get("action"))
PY
```

Also compare full-schema validity before and after against the old schema
from `origin/main`. "Every manifest that validated still validates" is the
claim to prove, and it is not the same as "the tests pass".
