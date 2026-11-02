---
category: Testing & coverage
---

# A new task-shape inventory needs its reading-reach floor owner and provenance tests updated

The Chinese A1 task-shape PR passed its focused schema test and assessment-artifact
check, but the Ubuntu package build failed after adding `task-shapes/a1.json`.
The reading-reach gate requires a `core/reading-reach-floor.d/chinese--a1.json`
owner for every new inventory, even when the measured floor is zero. A separate
Chinese exam-inventory test still expected the old phrase claiming that A1 task
shapes did not exist. Neither gate should be relaxed: measure the new row, add
its exact floor owner, and update the provenance assertion to require the new
truth while still checking that the two mocks remain absent. Include those
focused tests when a task shape changes, before trusting the broader CI build.
Before pinning a nonzero floor, check units and prompt bounds: a `minimum: 1`
on a 10-question reading part is a question count masquerading as a text-length
measure. Chinese's `items` stimulus-length convention counts Han characters
per prompt; the A1 project bounds are 2–10 for signs/forms and 12–30 for short
messages. A track-wide reach floor is not proof of A1-level mastery.
