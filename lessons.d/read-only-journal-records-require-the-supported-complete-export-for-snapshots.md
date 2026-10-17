---
category: Testing & coverage
---

# Read-only journal records require the supported complete export for snapshots

CLOC38's unchosen-child refusal controls attempted to serialize the read-only
JournalRecord slice directly. JournalRecord intentionally implements Deserialize
and Debug, not Serialize. The full producer passed 282 parser tests, then the
folder failed to compile with E0277; later suites never ran. A static review also
missed this trait requirement. Preserve that failed receipt without semantic-red
or passing-test credit.

Use CVLog::to_json_string(), the supported checked canonical export that includes
both graph entries and complete chronology. Compare parsed complete snapshots
before and after each checked refusal. This retains the intended exact equality
assertions without changing the library's public traits. Inspect the actual
snapshot API before writing a test, then require compiled terminal evidence.
