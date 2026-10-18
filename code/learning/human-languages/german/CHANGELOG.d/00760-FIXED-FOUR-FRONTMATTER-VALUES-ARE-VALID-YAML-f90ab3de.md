## Fixed — four frontmatter values are valid YAML

- GE-C01-guten-tag and GE-C02-heissen had an unquoted `etymology_hook:` with a colon and a space inside it (`etymology_hook: … a frozen accusative: the -en is …`), which a strict YAML parser reads as the start of a second key and rejects. Each is now one single-quoted scalar. The wording is unchanged.
- GE-C18-ja's `gloss: yes` and GE-C18-nein's `gloss: no` are now quoted (`'no'` and `'yes'`), so a YAML 1.1 parser no longer reads them as booleans. The repository's reader always kept them as text.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.

