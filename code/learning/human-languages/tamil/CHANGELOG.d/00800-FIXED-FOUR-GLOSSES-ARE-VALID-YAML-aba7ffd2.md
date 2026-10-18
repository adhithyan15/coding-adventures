## Fixed — four glosses are valid YAML

- TA-C23-naal, TA-C24-iravu, TA-C25-iniya-iravu and TA-C37-ivar-en-nanbar had a `gloss:` that opened with a quoted English word and went on past the closing quote (`gloss: "day" — …`), which a strict YAML parser rejects. Each is now one single-quoted scalar, with an apostrophe written `''`. The wording is unchanged, and the repository's reader returns the same text as before.
