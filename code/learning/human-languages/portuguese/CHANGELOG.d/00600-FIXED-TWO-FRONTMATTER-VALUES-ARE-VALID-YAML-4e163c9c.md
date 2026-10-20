## Fixed — two frontmatter values are valid YAML

- PT-C10-pais had an unquoted `gloss:` with a colon and a space inside it (`gloss: … worn-down of all: pater→pai, mater→mãe …`), which a strict YAML parser reads as the start of a second key and rejects. It is now one single-quoted scalar. The wording is unchanged.
- PT-C15-preterito-perfeito's `etymology_hook:` was double-quoted and held `\*`, which YAML reads as an unknown escape and rejects. It is now single-quoted, where a backslash stays literal, so the text is unchanged.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.

