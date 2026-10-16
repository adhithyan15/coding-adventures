---
category: Repo policy / workflow reminders
---

# Require affirmative review verdicts and complete object maps in delivery helpers

Independent stage delivery preflight withheld unexecuted helper versions: a
substring PASS could accept a withheld report, raw Rust/TOML hashes did not
prove complete native/Closure object maps, and a planner could accept exit0
without an emitted plan. Preserve rejected helpers and the refusal receipt.
Require affirmative non-withheld review verdicts, exact source attribution,
complete historical/native object maps and an actual validated emitted plan.
Recheck the clean exact head and pin the integrated frame to its full SHA.
No publication or source/test gate was executed through those defects.
