## Fixed — two frontmatter values are valid YAML

- AR-R22-the-head-and-the-hand and AR-R49-letters-second-pass had an unquoted `gloss:` with a colon and a space inside it (`gloss: … share one property: every root is …`), which a strict YAML parser reads as the start of a second key and rejects. Each is now one single-quoted scalar. The wording is unchanged.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.

