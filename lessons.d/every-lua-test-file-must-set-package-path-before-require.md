---
category: Lua
---

# Every Lua test file MUST set `package.path` before `require`

— even with rockspec installed:
```lua
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
```
This is NOT optional, especially on Windows CI where rockspec install does not put modules into the default search path. Re-learned multiple times.
