## Fixed — seven frontmatter values are valid YAML

- JA-W03-arigatou-read, JA-W03-dakuten, JA-W03-ka, JA-W03-u and JA-W06-hi-katakana had an unquoted `etymology_hook:` with a colon and a space inside it (`etymology_hook: … sign appears here: ありがとう is five …`), which a strict YAML parser reads as the start of a second key and rejects. Each is now one single-quoted scalar, with an apostrophe written `''`. The wording is unchanged.
- JA-C01-hai's `gloss: yes` and JA-W17-no's `romanization: no` are now quoted (`'no'` and `'yes'`), so a YAML 1.1 parser no longer reads them as booleans. The repository's reader always kept them as text.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.
