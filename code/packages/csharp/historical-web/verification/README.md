# HWEB10 checked request length

Run from the repository root with the pinned tools:

```sh
HWEB_GO=/workspace/.coding-adventures-tools/go/bin/go \
  /workspace/.coding-adventures-tools/python/cpython-3.13.17-linux-x86_64-gnu/bin/python3.13 \
  code/packages/csharp/historical-web/verification/sat_http09_length.py
```

The model follows `Http09.EncodeRequest` in
[`../Http09.cs`](../Http09.cs): a nonnegative `ReadOnlySpan<byte>.Length` is an
`Int32`, then the implementation evaluates `address.Length + 4 + 2` in a C#
`checked` block before allocating and copying the framed request. The SAT
model encodes the 32-bit addition of six separately from the successful
checked-add input domain (`1..Int32.MaxValue - 6`). It checks that a valid
length exists, that this input domain cannot produce a negative wrapped
length, and that removing the upper-bound guard admits such a witness.

This is an arithmetic model of the checked length expression, not a proof of
the C# compiler/runtime, span contents, array allocation success, maximum
array-size policy, memory availability, or the full encoder/parser behavior.
Those remain runtime properties outside this model.
