---
category: Ruby
---

# Standard native line endings conflict with the repository LF contract on Windows

Pinned Standard 1.57 reported a missing carriage return for every Ruby source in
an LF checkout. Its native EndOfLine rule requires CRLF on Windows, while this
repository's `.gitattributes` explicitly requires LF to keep fixtures stable.
An `extend_config` attempt did not override the existing Standard rule.

The two new benchmark gems exclude only Layout/EndOfLine in `.standard.yml`;
Git continues to enforce LF and every other Standard check remains active.
Invoke gem executables through the selected Ruby interpreter when Windows
wrapper scripts fail to locate `ruby.exe`. Bundler setup plus `Gem.bin_path`
loads the pinned executable without depending on a second shell's PATH.
