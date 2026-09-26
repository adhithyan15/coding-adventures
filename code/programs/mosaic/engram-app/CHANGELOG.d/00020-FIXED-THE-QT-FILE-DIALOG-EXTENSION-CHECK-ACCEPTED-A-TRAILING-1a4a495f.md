### Fixed — the Qt file-dialog extension check accepted a trailing newline

`fileFilter` decides whether a payload-supplied string "looks like an
extension" before it becomes a glob in the file dialog's filter. It used
`^\.?[A-Za-z0-9_-]{1,16}$` — and `QRegularExpression` is PCRE2, whose `$`
matches before a trailing newline unless `DollarEndOnlyOption` is set. So
`"apkg\n"` passed, and the check did not mean what its own comment said it
meant.

Measured against Qt 6 rather than assumed, because the answer is engine-specific
and does not transfer. On the build measured here PCRE2 conceded LF alone —
but that is a property of the build, not of the language: PCRE2's newline
convention is chosen at compile time, and a copy built with `ANYCRLF` or `ANY`
concedes `\r`, `\r\n`, NEL, LS and PS through `$` as well. So the hole was *at
least* LF-wide and possibly wider depending on whose PCRE2 Qt was linked
against. ICU answers the same question differently again (it concedes all of
them unconditionally), and Rust a third way — there `$` is end-of-haystack and
refuses every one, which is what the `[host_effects]` injection analysis turned
on.

Now `\z`, which is absolute: end of subject, no terminator concession on any
build or convention. Specifically `\z` and **not** `\Z` — PCRE2's `\Z` makes the
same concession `$` does, so it would have renamed the hole rather than closed
it.

`^` → `\A` in the same pattern fixes nothing today and is not pretending to:
`QRegularExpression` sets no `MultilineOption` by default, so `^` was already
start-of-subject. It is there so that turning that option on later cannot
reopen the other end.

Not exploitable: this application builds the payload, and nothing puts a newline
in an extension. It is worth fixing anyway because the failure it allowed is
**silent**. A glob of `*apkg\n` is refused nowhere downstream; it simply matches
no file, so the dialog opens showing nothing and reports no reason. A defensive
check whose comment overstates it is worse than no check, because the next
person reads the comment.

Verified by compiling the real `fileFilter` against Qt 6 and driving it: a
trailing LF now falls to the fallback filter, a good extension still survives
alongside a rejected one, and the previously-accepted cases are unchanged.

Scope, so the next reader does not over-read this: at the time of this fix Qt was
the only host with a shape check at all — Compose, Flutter and Electron normalise
instead, trimming and dot-prefixing whatever arrives — so there was no twin hole
to close alongside it.

*Superseded in part by the SwiftUI migration above*, which landed in the same
release: `EngramEffects.swift` now carries a shape check too, and it was written
with `\A`/`\z` from the start precisely because this fix had established that
`^`/`$` does not mean what it appears to. ICU concedes more terminators than
PCRE2 does, so the SwiftUI hole would have been the wider of the two.

The divergence that remains: Qt and SwiftUI now *reject* `".apkg\n"` where
Compose, Flutter and Electron *accept* it by trimming. Identical behaviour with
today's producers, which send static literals, but a real source of
host-specific filter differences if that ever stops being true.

