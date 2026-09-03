## HL-C313 — teaching a word can break a chapter you never touched

German chapter 14 taught *einen*, because *Ich habe einen Bruder* is the
hand-written chapter's own first example and *einen* was a headword nowhere in
the track. That one lesson failed a pinned assertion in
`continuity.test.ts` — about **chapter 1**, which the change did not touch:

    keeps German Chapter 1 free of untaught target-language previews
    expected [ { …(6) } ] to deeply equal []

`GE-C01-guten-tag` cites *ich wünsche einen guten Tag* to explain why the
greeting is *guten* and not *gut*. That citation had been previewing *einen*
eighty-nine lessons early since the chapter was written. **No gate could see
it**, because the forward-reference detector matches uses against HEADWORDS, and
nothing taught *einen*, so there was no headword to match.

### The general shape

**A forward-reference count is a function of what the corpus teaches, not of
what it says.** Every untaught word in the corpus is a preview the gate is blind
to, and the blindness lifts the moment some chapter teaches that word — which
means a migration can turn an old, unrelated chapter red without editing a line
of it.

Two consequences worth carrying:

* **Expect the count to move in both directions on every migration.** German
  went 36 -> 35 here as the net of four separate movements: two retired (the age
  sentence moved behind the copula it needed), two exposed (*Jahr* and *einen*
  became headwords), one of the exposed then fixed.
* **A newly exposed preview is a real defect, not an artefact of your change.**
  The instinct is to avoid teaching the word and keep the gate quiet. That
  preserves the bug and forfeits the lesson. Chapter 1 was rewritten to make its
  point in English — every teaching claim kept, the untaught German dropped.

### The pleasant version of the same effect

Exposure can also reveal a debt worth paying rather than a defect. `GE-C03-gehen`
names *Jahr* as one of three words hiding a silent lengthening *h*. Teaching
*Jahr* surfaced that as a forward reference — and the right response was not to
remove the mention but to make the new lesson **cash it in**: `GE-C14-jahr` now
requires `GE-SOUND-H-LENGTHEN-01` and says "here it is", closing a promise
chapter 3 had been carrying unspent.

That is the Root Ledger rule (`rootLedgerMinReuse`) arriving through a different
door: the forward-reference gate found an unspent setup that the root ledger
would also have flagged, and the fix for both is the same — spend it.

### Do not reach for the shared stash to answer "was this failing before?"

Diagnosing the above, the tempting move was `git stash -u`, run the suite on the
clean tree, `git stash pop`. The stash is **one list shared by every worktree in
the repo**, and the pop failed halfway: tracked changes came back, untracked
files did not ("already exists, no checkout"), leaving the index recording
pre-rename paths while the working tree held post-rename ones. Recovery was
`git add -A`, and the entry had to be dropped by explicit ref
(`git stash drop stash@{1}`) so as not to disturb two sibling agents' entries
sitting either side of it.

Answer that question with a second worktree, `git show <ref>:<path>`, or by
reasoning about the test — never by stashing.
