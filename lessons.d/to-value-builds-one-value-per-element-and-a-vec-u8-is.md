# `to_value` builds one Value per element, and a `Vec<u8>` is a lot of elements

`ok_with` in the wasm facade did `serde_json::to_value(value)` before rendering
to a string. `to_value` materialises a `Value` tree first — one `Value` per
array element — and `MediaAssetRecord.data` is a `Vec<u8>` serialising as an
array of decimal numbers. So one heap `Value` per media BYTE, on a target whose
entire address space is 4 GB.

Measured with a counting global allocator rather than repeating the estimate:

```
  1024 KiB media | to_value peak 34.0 MiB (34.0x)  |  streamed peak 2.8 MiB (2.8x)  |  12.4x less
```

34x peak per media byte, constant across 64 KiB / 256 KiB / 1 MiB — so it is
proportional, which is the property the issue asked to see demonstrated.
Streaming with `to_writer` straight into the output buffer takes it to 2.8x.
The residue is the numeric-array encoding itself; base64 is a separate change
with a real blast radius across every host.

**The interesting part was the part I got wrong.** I wrote that this was "not a
wire-format change" and pinned it with a byte-equality test against the old
construction. The test failed:

```
left:  {"ok":true,"state":{"id":..,"data":..,"nested":..}}
right: {"ok":true,"state":{"data":..,"id":..,"nested":..}}
```

`serde_json::Map` is a `BTreeMap` unless the `preserve_order` feature is on —
it is not, anywhere in this workspace — so `to_value` had been **sorting every
nested object's keys alphabetically**, and streaming emits declaration order.
Key order on the wire changed for every response the product makes.

It is safe, but "JSON objects are unordered" is a claim about consumers, not a
fact about this codebase, so I checked all five: `QJsonDocument`,
`jsonDecode`, `JsonDocument`, `JSONSerialization`, `org.json.JSONObject` — all
key-addressed, none indexing an object by position — then rebuilt and launched
a native host against the changed facade. The test now asserts semantic
equality and a second test pins the order change explicitly, so the next person
comparing responses as strings learns it from a test name instead of a diff.

Also fixed in passing: `unwrap_or(Value::Null)` turned a serialisation failure
into `{"ok":true,"state":null}` — a success-shaped response for a failed
operation. The Compose host already had `root.isNull("state")` in its failure
condition, which is what defending against a silent failure downstream looks
like when nobody fixed it upstream.
