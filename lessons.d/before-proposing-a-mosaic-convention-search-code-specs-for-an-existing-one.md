---
category: Mosaic compiler pipeline
---

# Before proposing a Mosaic convention, search code/specs for an existing one by capability, not by the name you would give it

**What went wrong.** UI87 (first revision, #15988) proposed `file.save` /
`file.open` as new standard effect kinds. `UI59-files-open-effect.md` already
made `files.open` a Mosaic convention and shipped handlers for it on XAML, Qt,
Compose and Flutter (`photo-picker-app`). UI87 was drafted from the two
implementations found by grepping for `file.save` (the browser executor, which
spells the kinds `file.*`, and Engram's handlers). The plural `files.open`
never matched. The note merged with a wrong framing and missed a naming split,
and needed a second revision.

**Do differently.**
- Search the spec index by *capability* before writing a convention:
  `ls code/specs | grep -i -E "file|effect|picker|dialog"`, not just the kind
  name you have in mind. UI numbers are not grouped by topic, so UI59 sits
  between a flexbox spec and a scroll spec.
- Where two spellings of one idea exist (`file.*` in the browser, `files.*` in
  UI59), say so in the note and pick one. Don't silently add a third.
