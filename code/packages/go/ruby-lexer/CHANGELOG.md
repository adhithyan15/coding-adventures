# Changelog

All notable changes to the ruby-lexer package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added
- `CreateRubyLexer()` function that loads the `ruby.tokens` grammar and returns a configured `GrammarLexer`
- `TokenizeRuby()` convenience function for one-shot tokenization of Ruby source strings
- Test suite verifying token count and keyword detection
