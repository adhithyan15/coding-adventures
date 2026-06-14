# @coding-adventures/macsyma-lexer

MACSYMA tokenizer backed by `code/grammars/macsyma/macsyma.tokens`, compiled to
TypeScript and statically linked into the package.

The runtime path is browser-safe: it does not read grammar files from disk.
