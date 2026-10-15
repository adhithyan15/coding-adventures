## Fixed — five frontmatter values are valid YAML

- MW-C23-counter-four's `etymology_hook:` and MW-R43-letters-second-pass's `gloss:` had an unquoted value with a colon and a space inside it (`etymology_hook: Ask, react, counter: the first exchange …`), which a strict YAML parser reads as the start of a second key and rejects. Each is now one single-quoted scalar. The wording is unchanged.
- MW-C33-hear-no-nine's `headword: no`, MW-C33-hear-no-nine's `romanization: no` and MW-C33-no-nine's `romanization: no` are now quoted (`'no'`), so a YAML 1.1 parser no longer reads them as booleans. The repository's reader always kept them as text.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.
