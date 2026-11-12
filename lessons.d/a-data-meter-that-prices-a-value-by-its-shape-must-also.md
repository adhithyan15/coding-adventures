---
category: Security boundaries
---

# A data meter that prices a value by its shape must also count the heap strings the value owns, or copies outrun the charge

The Warrant v1 data meter (VER04k section 2.7) priced a value by its shape:
32 bytes per slot plus the payload of `Bytes`, and 0 for a nullary constructor
or an `E.CASE` error value, because "its storage is the slot that holds it".
But the ground evaluator's `EvalValue::Con { ty: String, ctor: String, .. }`
and `Result(Err((String, String)))` own their names on the heap, and every copy
re-allocates them. `replicate(<10,000-byte constructor>, 65536)` was admitted
with 2 MiB charged and requested 661,727,185 bytes (the security review saw a
1.3 GB peak), so the "64 MiB per value" claim was false for any document with
long names.

The fix (VER04k E20) counts the stored name bytes in `dsize` and caps v1
identifiers at 128 bytes. The test that would have caught it is a counting
`#[global_allocator]` that compares the bytes really requested against the
meter's charge for each constructing form (`tests/wxd_alloc_probe.rs`), not a
table of expected charges, which only restates the cost model's own blind spot.

What to do differently: when a cost model is written against a data type,
walk the type's definition field by field and ask what each one allocates on a
copy (`String`, `Vec`, `Box`), then hold the meter to a real allocator
measurement. A per-value cap also says nothing about reads: values that are
deep-copied on every read and charged nothing still bound total memory only by
the number of reads (VER04k note on E4, backlog WX-RD1).
