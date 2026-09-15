---
category: Ruby
---

# `bundle install` requires `mise.toml`-managed Ruby

(project requires 3.4+; system Ruby is 2.6.10). Locally: rely on mise shims (no `mise exec --` prefix). Building Ruby 3.4 from source on macOS needs `brew install libyaml` and `RUBY_CONFIGURE_OPTS="--with-libyaml-dir=/opt/homebrew"` on Apple Silicon; mise's `ruby.compile=false` does not yet use precompiled binaries.
