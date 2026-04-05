# Scytale Cipher (Lua)

Ancient Spartan transposition cipher implementation in Lua.

## Usage

```lua
local scytale = require("coding_adventures.scytale_cipher")

local ct = scytale.encrypt("HELLO WORLD", 3)
-- => "HLWLEOODL R "

local pt = scytale.decrypt(ct, 3)
-- => "HELLO WORLD"

local results = scytale.brute_force(ct)
-- => {{key=2, text="..."}, {key=3, text="HELLO WORLD"}, ...}
```

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.
