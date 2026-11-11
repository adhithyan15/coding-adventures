---
category: Repo policy / workflow reminders
---

# Read a generated lesson shard before replacing its placeholder

The lesson generator emits a full explanatory placeholder, not the shorter
`TODO: Fill in the lesson.` text guessed by an initial patch. The mismatch made
the patch fail safely, but cost a round trip. Read the generated shard before
constructing its replacement patch and match the exact placeholder that is
present rather than assuming generator output.
