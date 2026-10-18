## Fixed — four frontmatter values are valid YAML

- ZH-C04-practice, ZH-W03-yi, ZH-W16-yu-observe and ZH-W18-kan-observe had an unquoted `etymology_hook:` with a colon and a space inside it (`etymology_hook: … is taught here: two characters came …`), which a strict YAML parser reads as the start of a second key and rejects. Each is now one single-quoted scalar. The wording is unchanged.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.

