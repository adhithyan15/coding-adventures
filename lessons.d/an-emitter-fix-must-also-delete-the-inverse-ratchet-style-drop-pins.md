---
category: Testing & coverage
---

# An emitter fix must also delete the inverse-ratchet style-drop pins it obsoletes in every package

**What went wrong.** #17915 taught the Qt emitter to emit text widths. Each
mosaic package's `tests/native_complete_gate.rs` keeps an
`ALLOWED_STYLE_DROPS` list, and an inverse ratchet
(`no_pinned_style_drop_has_silently_been_fixed`) that fails once a pinned
drop stops happening. `mosaic-pkg-deck-stats` still pinned
`(Backend::Qt, "width")`. #17915 never rebuilt that package, so it merged
green. Every later PR whose shard built deck-stats then failed, as #17824's
shard 9 did.

**Fix.** Remove the pin, after checking each package that pins it.
engram-app still drops a Qt `width` and keeps its pin.

**Do differently.** When an emitter starts mapping a property, grep for its
pins, `git grep -n '(Backend::<Emitter>, "<property>")' code`, then run
each match's `native_complete_gate` test.
