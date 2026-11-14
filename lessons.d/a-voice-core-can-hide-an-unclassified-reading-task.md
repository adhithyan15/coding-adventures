---
category: Testing & coverage
---

# A voice core can hide an unclassified reading task

The Kannada A1 named-place lessons generated `coreDrivable: true` while their
spoken core still said “follow the printed line with your finger,” “read both
lines,” and “listen, then read.” The narration copied those instructions,
although the modality manifest detached only the Writing sections. The
existing classifier and green generation checks did not catch these phrasings;
an independent safety review did.

The lessons now ask for spoken recall and answers without a page or gesture.
Keep writing and print-check tasks in the detachable Writing section. For a
new voice-core lesson, inspect the generated narration as well as the
modality flag, and test the core prose for visual or hands-on imperatives the
classifier might not yet recognize. Track a general detector expansion
separately so the local wording fix does not weaken any existing gate.
