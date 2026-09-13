# A "drive this to zero" programme meets a validator that has never seen zero

The last hand-written chapter in a 1,330-chapter corpus was generated, which
emptied `core/book-generation.d/handwritten.d/`. Git cannot track an empty
directory, so the directory left the tree — and the owner-root validator, which
asserts the root holds EXACTLY `_meta.json` plus every section directory, failed
with "missing, legacy, or unexpected entries".

It passed locally and failed in CI, for the reason that pairing always has:
`git rm` leaves the now-empty directory sitting on the authoring machine, and
only a fresh checkout sees that it is gone. **The empty case had been unreachable
for as long as any track had one hand-written chapter, so nothing had ever
exercised it — and the programme's whole purpose was to reach it.**

Three things generalise.

**A long-running "reduce X to zero" effort should test the zero before it gets
there.** The terminal state of such a programme is by construction the one state
no fixture covers, and it arrives exactly once, in the PR that has the least
appetite for a surprise.

**"Must exist" and "may be empty" are two claims, and a fix must not merge
them.** The tempting repair was to let an absent section read as empty. That
would trade loud for silent: a genuinely deleted `targets.d` would then produce a
book with no generated chapters and no error. The fix is a tracked `.gitkeep`
that holds the directory open plus one filter in the reader, so *missing* still
throws and *empty* becomes expressible.

**Two readers of one contract drift apart one fix at a time.** The TypeScript
loader and `data/scripts/sharded_ledger.py` both enforce this shape. Fixing the
TypeScript side made the Python side reject a tree the TypeScript side had just
accepted, so the counter it computes went from "zero, correctly" to a crash. Both
now filter the placeholder, both carry a comment pointing at the other, and both
test suites pin the empty case AND the absent case.
