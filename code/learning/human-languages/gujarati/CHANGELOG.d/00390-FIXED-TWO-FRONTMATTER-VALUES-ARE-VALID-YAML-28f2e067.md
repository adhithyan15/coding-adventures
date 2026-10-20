## Fixed — two frontmatter values are valid YAML

- GU-C29-nadi's `etymology_hook:` and GU-R45-letters-second-pass's `gloss:` had an unquoted value with a colon and a space inside it (`etymology_hook: … on the first: the river is …`), which a strict YAML parser reads as the start of a second key and rejects. Each is now one single-quoted scalar. The wording is unchanged.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.
