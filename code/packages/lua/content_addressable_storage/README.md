# coding-adventures-content-addressable-storage

Pure Lua implementation of generic **Content-Addressable Storage (CAS)**.

CAS maps the *hash of content* to the content itself.  The hash is
simultaneously the address and the integrity check — if the bytes returned by
the store don't hash to the key you asked for, the data is corrupt.  No
separate checksum file or external trust anchor is needed.

This is exactly how Git stores its objects: every blob, tree, commit, and tag
is stored by the SHA-1 hash of its serialized bytes.

## Architecture

```
┌──────────────────────────────────────────────────┐
│  ContentAddressableStore                          │
│  · put(data)          → 20-byte sha1 key         │
│  · get(key)           → data (integrity verified) │
│  · find_by_prefix(hex)→ full key                 │
└─────────────────┬────────────────────────────────┘
                  │ BlobStore (abstract base class)
         ┌────────┴──────────────────────────────┐
         │                                       │
  LocalDiskStore                    (future backends)
  root/XX/XXXXXX…
```

## Usage

```lua
local cas = require("coding_adventures.content_addressable_storage")

-- Create a filesystem-backed store
local disk = cas.LocalDiskStore.new("/tmp/myrepo")
local db   = cas.ContentAddressableStore.new(disk)

-- Store some data — key is the SHA-1 hash of the data
local key = db:put("hello, world")
print(cas.key_to_hex(key))
-- → "a0b65939670bc2c010f4d5d6a0b3e4e4b0b3a3a9" (example)

-- Retrieve and verify
local data = db:get(key)
print(data)  -- → "hello, world"

-- Check existence
print(db:exists(key))   -- → true

-- Find by abbreviated hex prefix (like `git show a3f4b2`)
local found, err = db:find_by_prefix("a0b65939")
-- found == key  (if unique match)

-- Hex utilities
local hex = cas.key_to_hex(key)        -- "a0b6…" (40 chars)
local k2  = cas.hex_to_key(hex)        -- back to 20-byte binary
```

## Error Handling

Methods return `(value, nil)` on success and `(nil, err_table)` on failure.
The `err_table.type` field is the primary discriminator:

| `type`             | Meaning                                           |
|--------------------|---------------------------------------------------|
| `"not_found"`      | Key is not in the store                           |
| `"corrupted"`      | Stored bytes don't hash to the requested key      |
| `"ambiguous_prefix"` | Hex prefix matches two or more objects          |
| `"prefix_not_found"` | Hex prefix matches zero objects                 |
| `"invalid_prefix"` | Hex string is empty or contains non-hex chars     |
| `"store_error"`    | Backend I/O failure                               |

## LocalDiskStore Path Layout

Objects are stored using Git's 2/38 fanout layout:

```
root/
  a3/
    f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5   ← 38-char hex remainder
  fe/
    9a3b…
```

The first byte of the SHA-1 hash becomes the 2-char directory name; the
remaining 19 bytes form the 38-char filename.  This limits directory entries
to ~1/256 of the total object count, matching Git's original design.

Writes are atomic: data goes to a temp file first, then `os.rename()` moves it
into place.

## Custom Backends

Subclass `BlobStore` and override the four abstract methods:

```lua
local MyStore = setmetatable({}, { __index = cas.BlobStore })
MyStore.__index = MyStore

function MyStore.new()
    local self = cas.BlobStore.new()
    setmetatable(self, MyStore)
    return self
end

function MyStore:put(key, data)  ... return true end
function MyStore:get(key)        ... return data  end
function MyStore:exists(key)     ... return bool  end
function MyStore:keys_with_prefix(prefix) ... return list end
```

## Dependencies

- Lua 5.4+
- `coding-adventures-sha1` (pure Lua SHA-1 from this repo)

## Running Tests

```sh
cd tests
LUA_PATH="../sha1/src/?.lua;../sha1/src/?/init.lua;../src/?.lua;../src/?/init.lua;;" \
  busted . --verbose --pattern=test_
```
