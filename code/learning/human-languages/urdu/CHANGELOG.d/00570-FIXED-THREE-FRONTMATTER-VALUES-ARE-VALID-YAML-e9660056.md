## Fixed — three frontmatter values are valid YAML

- UR-R37-letters-second-pass and UR-R99-letters-second-pass had an unquoted `gloss:` with a colon and a space inside it (`gloss: second pass: the same sixteen …`), which a strict YAML parser reads as the start of a second key and rejects. Each is now one single-quoted scalar. The wording is unchanged.
- UR-C01-ji-han's `gloss: yes` is now quoted (`'yes'`), so a YAML 1.1 parser no longer reads it as a boolean. The repository's reader always kept it as text.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.
