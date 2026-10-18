## Fixed — two frontmatter values are valid YAML

- FR-C18-oui's `gloss: yes` and FR-C37-on's `headword: on` are now quoted (`'on'` and `'yes'`), so a YAML 1.1 parser no longer reads them as booleans. The repository's reader always kept them as text.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.

