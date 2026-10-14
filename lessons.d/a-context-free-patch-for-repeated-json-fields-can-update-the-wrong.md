---
category: Repo policy / workflow reminders
---

# A context-free patch for repeated JSON fields can update the wrong exam point

An inventory repair replaced the first matching `"probe": null` in a large
JSON file while the nearby context in the patch only identified the replacement
text, not the owning point. The new spatial atoms landed on `SA-A1-POS-02`
(plural possession) instead of `SA-A1-SP-02` (relative position). The total
coverage count rose by one, so an aggregate-only check and the completion plan
both looked correct even though the wrong point was now credited.

The category assertion caught the error: total coverage was 143, but `Spatial
notions` remained 3/4. Inspecting `git diff` then showed the unrelated possession
point had changed.

When patching a repeated field in a large JSON inventory, include the record's
stable `id` and the field being replaced in the same patch hunk. Afterward,
inspect the diff before trusting aggregate measurements, and pin both the exact
point and its category. A right total can be the sum of the wrong evidence.
