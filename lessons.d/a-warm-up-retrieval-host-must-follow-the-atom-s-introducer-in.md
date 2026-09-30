---
category: Repo policy / workflow reminders
---

# A warm-up retrieval host must follow the atom's introducer in curriculum.json path order and stay under the 300-second duration budget

**What went wrong.** The Spanish A2 reinforcement pass added one-line Warm-up
retrievals to later lessons (`add_practice.py`, which also adds the atom's
introducing lesson to the host's `prerequisites`). Hosts were picked from the
*reading order* (`readingOrder`). Spanish's `curriculum.json` path order is a
different order, and the validator requires every prerequisite to precede its
dependant *there*. Two hosts (`ES-C09-estamos` for `ES-C09-ista`, and
`ES-C43-casa` for `ES-C10-futuro-simple`) came before their introducers in path
order, so `integration.test.ts` and `cli.test.ts` failed with "X must precede Y
in curriculum.json". Two more hosts (`ES-C268-donde-esta`, `ES-C41-creer`) were
already close to the budget, and the extra line pushed their effective duration
past 300 seconds (`schema-v2-duration-budget`).

**Fix.** Each retrieval moved to a host that comes after the introducer in the
curriculum path and has room in its duration budget.

**Next time.** Before adding a retrieval, check two things about the host:
1. its position in the track's curriculum path (concatenate `path[].lessons`)
   is after the introducer's, not only its position in reading order;
2. `estimateLessonDuration(host).effectiveSeconds` stays well under 300 after
   the line is added (a one-line retrieval adds roughly 15-25 seconds).
