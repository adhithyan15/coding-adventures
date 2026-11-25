---
category: Lua
---

# Run Lua syntax checks only on Lua source files

`luac -p` parses Lua code only. Keep C extension compilation on the C
compiler's path and pass only `.lua` files to `luac`, so a failed Lua parse
does not get confused with native build evidence.
