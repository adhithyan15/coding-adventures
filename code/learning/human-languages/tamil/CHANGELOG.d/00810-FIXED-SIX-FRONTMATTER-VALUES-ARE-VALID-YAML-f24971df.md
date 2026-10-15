## Fixed — six frontmatter values are valid YAML

- TA-C20-pathinondru-irupathu, TA-C20-vaanilai, TA-C27-maalai, TA-C28-pirpakal, TA-C30-maalai-vanakkam and TA-R112-letters-second-pass had an unquoted `gloss:` with a colon and a space inside it (`gloss: … out the arc: all four Dravidian …`), which a strict YAML parser reads as the start of a second key and rejects. Each is now one single-quoted scalar, with an apostrophe written `''`. The wording is unchanged.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.
