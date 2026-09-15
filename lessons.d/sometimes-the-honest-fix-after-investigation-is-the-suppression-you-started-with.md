# Sometimes the honest fix after investigation IS the suppression you started with — don't force a "real" fix past what the evidence supports

Follow-on to the `embeddable-http-server` correction above. The instinct from
"investigate before suppressing" is to keep digging until you find a positive
fix. But investigation can just as validly conclude "the exclusion was correct,
the capability really is incomplete, and fixing it properly is out of scope for
this change" — and forcing a deeper fix at that point (e.g. hand-rolling a
Windows fan-out acceptor inside a CI-rescue with no way to test it before a
slow, blind CI round trip) trades a well-understood, honestly-labeled gap for
an *unverified* one. The deliverable of "investigate, don't just suppress" is
not always a positive change — sometimes it's a suppression with a comment that
correctly explains the boundary of what was and wasn't fixed, and a decision
about scope that a human reading the commit message can evaluate and revisit.
