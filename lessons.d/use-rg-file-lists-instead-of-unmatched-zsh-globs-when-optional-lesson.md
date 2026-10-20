---
category: Repo policy / workflow reminders
---

# Use rg file lists instead of unmatched zsh globs when optional lesson topics may be absent

A loop over several optional `lessons.d/*topic*` globs stopped immediately when
zsh found no Sanskrit-named lesson and treated the unmatched pattern as an
error. Build the candidate list with `rg --files lessons.d | rg 'topic|other'`
and iterate over those concrete paths instead. Optional discovery should yield
an empty list, not abort the rest of the preflight reading.
