# Scytale Cipher (Lua)

Ancient Spartan transposition cipher implementation in Lua.

This package follows [CR02](../../../specs/CR02-scytale-cipher.md). Grid cells are UTF-8 decoded Unicode scalar values, uneven ciphertext columns are reconstructed explicitly, and only trailing U+0020 padding is removed. `brute_force` rejects inputs above 4096 scalars before allocating quadratic candidate output. Production code is deterministic pure computation with no OS capabilities.

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
