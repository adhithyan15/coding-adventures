---
category: Supply chain & CI pinning
---

# A Python dev dependency must be on the approved allowlist; name an approved package that requires it instead

**What went wrong.** CV2-S6a added `coverage>=7.9` to `python/warrant-corpus`'s
`dev` extras so that the `[tool.coverage.run] core = "sysmon"` setting is understood.
The "Repo-wide metadata contracts" job then failed: `third_party_dependency_policy.py`
refuses any Python dependency name that is not in
`code/specs/data/third-party-dependencies/python-allowlist.json` ("New dependency names
require explicit review: coverage"). `coverage` is not on that list, although `pytest-cov`
is, and the approved set may only shrink without a review.

**Fix.** Do not name the package directly; raise the floor of an approved package that
requires it. `pytest-cov>=7.1` requires `coverage[toml]>=7.10.6`, which is newer than the
version the setting needs.

**Next time.** Before adding any third-party dependency, run
`python3 code/scripts/third_party_dependency_policy.py check --ecosystem python`
(and the matching ecosystem for other languages). If the name is genuinely new, it needs
the owner's explicit review, not a quiet allowlist edit.
