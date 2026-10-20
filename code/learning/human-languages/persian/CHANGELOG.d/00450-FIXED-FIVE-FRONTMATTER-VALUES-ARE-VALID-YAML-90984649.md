## Fixed — five frontmatter values are valid YAML

- FA-C04-chetor's `etymology_hook:`, FA-C14-mard's `etymology_hook:` and FA-R26-letters-second-pass's `gloss:` had an unquoted value with a colon and a space inside it (`etymology_hook: … Arabic-loan tor “manner”: literally “in what …`), which a strict YAML parser reads as the start of a second key and rejects. Each is now one single-quoted scalar. The wording is unchanged.
- FA-C01-bale's `gloss: yes` and FA-C01-na's `gloss: no` are now quoted (`'no'` and `'yes'`), so a YAML 1.1 parser no longer reads them as booleans. The repository's reader always kept them as text.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.

