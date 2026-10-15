## Fixed — ten glosses are valid YAML

- TE-C23-roju, TE-C24-ratri, TE-C25-shubha-ratri, TE-C26-udayam, TE-C27-sayantram, TE-C28-madhyahnam-widened, TE-C29-subhodayam, TE-C30-subha-sayantram and TE-C31-subha-madhyahnam had a `gloss:` that opened with a quoted English word and went on past the closing quote (`gloss: "day" — …`), which a strict YAML parser rejects. Each is now one single-quoted scalar, with an apostrophe written `''`. The wording is unchanged.
- TE-C51-salutation's `gloss: "a salutation, "my respects""` is now `gloss: 'a salutation, "my respects"'`, the text the repository's reader already returned. All ten read exactly as before.
