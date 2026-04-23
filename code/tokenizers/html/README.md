# HTML Tokenizer Definitions

This directory holds authoring artifacts for generated HTML tokenizers.

Tokenizer definition files are build-time inputs. Venture and other production
runtimes should link generated source code rather than loading these TOML files
from disk.

## Files

- `html-skeleton.tokenizer.states.toml`: the first tokenizer-profile authoring
  artifact. It mirrors the current Rust HTML skeleton tokenizer so the next
  implementation slice can prove `.tokenizer.states.toml -> generated Rust ->
  statically linked tokenizer` end to end.

