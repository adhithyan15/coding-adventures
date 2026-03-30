# coding-adventures-uuid (Lua)

Pure Lua implementation of UUID v1, v3, v4, v5, and v7 generation and parsing,
following RFC 4122 / ITU-T X.667.

## What Is a UUID?

A UUID (Universally Unique Identifier) is a 128-bit value used to identify
information in computer systems without central coordination. Its canonical
form is 32 hexadecimal digits in five groups:

```
xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx
```

Where M is the version (1–7) and N encodes the variant (RFC 4122 = `8`, `9`, `a`, or `b`).

## Versions Implemented

| Version | Algorithm                        | Sortable | Deterministic |
|---------|----------------------------------|----------|---------------|
| v1      | Time + random node (48-bit)      | No       | No            |
| v3      | MD5(namespace + name)            | No       | Yes           |
| v4      | 122 random bits                  | No       | No            |
| v5      | SHA-1(namespace + name)          | No       | Yes           |
| v7      | 48-bit Unix ms + random bits     | Yes      | No            |

## Usage

```lua
local uuid = require("coding_adventures.uuid")

-- v4: random UUID (most common)
print(uuid.generate_v4())
-- "3d813cbb-47fb-32ba-91df-831e1593ac29"  (example)

-- v5: deterministic from a name (preferred over v3)
print(uuid.generate_v5(uuid.NAMESPACE_DNS, "www.example.com"))
-- "2ed6657d-e927-568b-95e3-af9f787f5a91"  (always this exact value)

-- v3: deterministic from a name (MD5 variant)
print(uuid.generate_v3(uuid.NAMESPACE_DNS, "www.example.com"))
-- "5df41881-3aed-3515-88a7-2f4a814cf09e"  (always this exact value)

-- v7: time-sortable random UUID
print(uuid.generate_v7())
-- "018e3d8a-b2c0-7abc-8def-1234567890ab"  (example)

-- validate
print(uuid.validate("550e8400-e29b-41d4-a716-446655440000"))  -- true
print(uuid.validate("not-a-uuid"))                             -- false

-- parse
local info = uuid.parse(uuid.generate_v4())
print(info.version)  -- 4
print(info.variant)  -- "rfc4122"

-- nil UUID
print(uuid.nil_uuid())  -- "00000000-0000-0000-0000-000000000000"
```

## Well-Known Namespaces

```lua
uuid.NAMESPACE_DNS  -- "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
uuid.NAMESPACE_URL  -- "6ba7b811-9dad-11d1-80b4-00c04fd430c8"
uuid.NAMESPACE_OID  -- "6ba7b812-9dad-11d1-80b4-00c04fd430c8"
uuid.NAMESPACE_X500 -- "6ba7b814-9dad-11d1-80b4-00c04fd430c8"
```

## RFC 4122 Test Vectors

The following test vectors from RFC 4122 Appendix B are verified by the test suite:

```
v3(NAMESPACE_DNS, "www.example.com") = "5df41881-3aed-3515-88a7-2f4a814cf09e"
v5(NAMESPACE_DNS, "www.example.com") = "2ed6657d-e927-568b-95e1-2665a8aea6a2"
```

## Installation

```bash
# Install dependencies first
cd ../md5  && luarocks make --local coding-adventures-md5-0.1.0-1.rockspec
cd ../sha1 && luarocks make --local coding-adventures-sha1-0.1.0-1.rockspec
# Then install uuid
cd ../uuid && luarocks make --local coding-adventures-uuid-0.1.0-1.rockspec
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

## Dependencies

- `coding-adventures-md5` — used by `generate_v3`
- `coding-adventures-sha1` — used by `generate_v5`
