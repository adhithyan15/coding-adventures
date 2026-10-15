## Fixed — the MR-C01-yeto gloss is valid YAML

- MR-C01-yeto's `gloss: "I'll be going" (yeto m. / yete f.)` opened with a quoted phrase and went on past the closing quote, which a strict YAML parser rejects. It is now one single-quoted scalar, with the apostrophe written `''`. The wording is unchanged, and the repository's reader returns the same text as before.
