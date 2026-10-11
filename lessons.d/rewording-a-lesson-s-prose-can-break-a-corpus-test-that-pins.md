---
category: Testing & coverage
---

# Rewording a lesson's prose can break a corpus test that pins that prose, so run the full suite, not targeted files

**What went wrong.** The Devanagari ā/phrases change reworded
SA-W03-mama-nama-guided-copy so that it follows its new filmstrip. Disk was
short, so only targeted human-language-data suites ran locally (figure,
figure-targets, integration, narration, modality, book). CI's full run then
failed `tests/corpus/sanskrit/pins-sanskrit-s-pre-a1-writing-ladder-*.case.ts`,
which pins phrases from that lesson's Markdown (`"न + ◌ा"`).

**Fix.** The pin now checks the new wording (`**न**, the stem of **◌ा**, then
**म**`), which carries the same teaching point: the sign is part of the word.

**Do differently.** Any change that edits lesson prose (strip rewording, guard
fixes) must run the FULL human-language-data suite, or at least every
`tests/corpus/<track>*` file for the tracks it touches. The per-track corpus
pins quote lesson Markdown verbatim, and no figure or narration test covers
them. If disk is short, free it (remove merged, clean worktrees) rather than
narrowing the run.
