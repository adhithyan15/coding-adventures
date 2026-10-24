---
category: Testing & coverage
---

# Vocabulary tranches screened before the core lands must be screened again after it

**What went wrong.** The four Kannada B1 vocabulary tranches (chapters 319–546) were drafted and reviewed while the B1 core (chapters 309–318) was still being written. Each tranche passed the pre-generation lint against the words the track taught at the time. Once the core was in the worktree, the same lint failed on four words in two tranches:
- review cues the core had taken: ಆಸನ "a seat" against the core's ಸೀಟು "a seat", and ಗೋಲು "a goal" against the core's ಗುರಿ "a goal, an aim";
- close-variant hits on the core's new forms: ಮಳಿಗೆ "a shop" against ಮಳೆ "rain" (taught inside ಮಳೆ ಬಂದರೆ), and ಹೊದೆ "to wrap oneself" against ಹೋದೆ "I went".

The core's headwords also include its grammar headwords, so it adds forms such as ಹೋದೆ and ಮಳೆ that no word list predicted.

**The fix.** The two cues were narrowed ("a formal seat", "a goal in sport"), and the two different words were explained in the spec's `ALLOW`, with a note telling ಹೊದೆ apart from ಹೋದೆ.

**Do differently.** Treat the core's landing as a change to the screening data. Rebuild the taught-word, cue and forward-reference data from the worktree, then re-run the lint and the forward check on every tranche before generating anything. Generation scripts should do this rebuild themselves, not only check that the core files exist.
