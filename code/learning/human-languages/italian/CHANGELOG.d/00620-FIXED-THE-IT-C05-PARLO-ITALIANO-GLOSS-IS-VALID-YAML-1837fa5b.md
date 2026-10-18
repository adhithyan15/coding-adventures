## Fixed — the IT-C05-parlo-italiano gloss is valid YAML

- IT-C05-parlo-italiano had a `gloss:` that opened with a quoted English word and went on past the closing quote (`gloss: "day" — …`), which a strict YAML parser rejects. Each is now one single-quoted scalar, with an apostrophe written `''`. The wording is unchanged. The repository's reader returns the same text as before.

