## Fixed — two frontmatter values are valid YAML

- IT-C12-numeri-17-20 had an unquoted `gloss:` with a colon and a space inside it (`gloss: … order at 17: the ten jumps …`), which a strict YAML parser reads as the start of a second key and rejects. It is now one single-quoted scalar. The wording is unchanged.
- IT-C29-no's `headword: no` is now quoted (`'no'`), so a YAML 1.1 parser no longer reads it as a boolean. The repository's reader always kept it as text.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.

