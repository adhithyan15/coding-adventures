---
category: Testing & coverage
---

# Absent upstream files do not establish an owned diagnostic repair

A pending-main static audit incorrectly required the owned macOS diagnostic
helper/test to be nonempty upstream. Both upstream sides omit those owned-only
files. Preserve the failed assertion receipt; verify actual nonempty owned
mode/type/blob entries and the complete disjoint pending path overlay instead.
Two absent entries prove only absence, never delivery or execution of a repair.
The accepted bounded bridge records this distinction without merging code.
