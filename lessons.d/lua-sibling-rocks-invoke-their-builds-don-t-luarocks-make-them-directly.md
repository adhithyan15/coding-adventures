---
category: Lua
---

# Lua sibling rocks: invoke their `BUILD`s, don't `luarocks make` them directly

— they may depend on other unpublished local rocks. For grammar-driven lexer tests, prepend sibling `src/` dirs to `package.path` so in-repo `.tokens` files resolve over installed rocks.
