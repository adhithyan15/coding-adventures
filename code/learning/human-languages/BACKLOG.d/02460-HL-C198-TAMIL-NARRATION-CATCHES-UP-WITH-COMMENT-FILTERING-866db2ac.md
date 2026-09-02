## HL-C198 — Tamil narration catches up with comment filtering

Regenerating narration for the Hindi apology metadata repair exposed a stale
artifact left by the repository-wide author-comment filter. The generator now
correctly removes HTML author comments from speech, and the Spanish fixtures
that motivated that repair were regenerated, but Tamil chapter 65 still
contained two comments in its checked-in narration and hash.

Regenerate Tamil chapter 65 with the current narration generator and commit the
updated JSON, text, and hash beside the Hindi metadata repair that surfaced the
drift. This closes the discovered generator mismatch rather than allowing the
next clean narration check to rediscover it.
