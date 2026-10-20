## Fixed — two frontmatter values are valid YAML

- PA-C35-jaldi-milaange and PA-C36-shubh-raat had an unquoted `etymology_hook:` with a colon and a space inside it (`etymology_hook: … Punjabi always uses: the adverb first, …`), which a strict YAML parser reads as the start of a second key and rejects. Each is now one single-quoted scalar, with an apostrophe written `''`. The wording is unchanged.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.
