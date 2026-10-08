---
category: Testing & coverage
---

# An exact ratchet ledger loses its directory with its last file, and the reader must treat that as no debt

**What went wrong.** `human-language-data/tests/drivable-writing-cues.test.ts`
keeps one debt file per track in `tests/drivable-writing-debt/`, and its
header promised that "a track with no debt has no file at all". The Marwadi
drive-debt fix deleted the last ledger file. Git cannot track an empty
directory, so the directory left the tree with it. `loadDebt()` called
`readdirSync` on it at module load, which threw ENOENT, and the test file
crashed before any test ran. The fix's own commit message said "the test reads
a missing ledger file as no debt", and that was true per file but not for the
directory. Its anti-vacuity check, `offenders.size > 0`, would also have
failed at zero, because it required debt to exist.

**The fix.** A missing directory reads as "no track has debt". For this
ledger that is safe, unlike the section directory in "A drive this to zero
programme meets a validator that has never seen zero", because the ledger is
EXACT: if the directory is lost by mistake while debt remains, every
offending lesson is reported as NEW debt. The anti-vacuity check now asks the
detector to keep firing on the non-drivable lessons, where writing is
legitimate, rather than asking for debt.

**Do differently.** When a ratchet's goal is zero, test the zero state before
the PR that reaches it: delete the whole ledger directory locally and run the
test. Also check every anti-vacuity assertion: "some debt exists" is the one
claim the programme is trying to make false.
