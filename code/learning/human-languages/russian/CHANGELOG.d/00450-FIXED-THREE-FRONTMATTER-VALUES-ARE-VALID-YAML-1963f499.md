## Fixed — three frontmatter values are valid YAML

- RU-R28-letters-second-pass, RU-R81-letters-second-pass and RU-R93-letters-second-pass had an unquoted `gloss:` with a colon and a space inside it (`gloss: second pass: the same one …`), which a strict YAML parser reads as the start of a second key and rejects. Each is now one single-quoted scalar. The wording is unchanged.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.

