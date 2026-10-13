---
category: Testing & coverage
---

# Listening duration counts must exclude unspoken speaker labels

After previews and scoring markers were excluded, the C1 duration test still
counted Devanagari speaker-name labels inside the scored slice as if speakers
said their own names before every turn. Six recording masters passed only with
that production metadata. Mark speaker identities as explicitly unspoken
metadata, strip every label before counting, and expand the actual utterance
body until its own token count meets the duration band.
