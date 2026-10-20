## Fixed — the PT-C05-falo-portugues gloss is valid YAML

- PT-C05-falo-portugues had a `gloss:` that opened with a quoted English word and went on past the closing quote (`gloss: "day" — …`), which a strict YAML parser rejects. Each is now one single-quoted scalar, with an apostrophe written `''`. The wording is unchanged. The repository's reader returns the same text as before.

