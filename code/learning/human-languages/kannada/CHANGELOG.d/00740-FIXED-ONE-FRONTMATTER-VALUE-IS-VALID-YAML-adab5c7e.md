## Fixed — one frontmatter value is valid YAML

- KA-C20-havamana had an unquoted `gloss:` with a colon and a space inside it (`gloss: … Persian+Sanskrit HYBRID compound: hava ("air," from …`), which a strict YAML parser reads as the start of a second key and rejects. It is now one single-quoted scalar. The wording is unchanged.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.
