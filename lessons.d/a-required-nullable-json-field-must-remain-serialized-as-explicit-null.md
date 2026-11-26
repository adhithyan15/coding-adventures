---
category: C#
---

# A required nullable JSON field must remain serialized as explicit null

The shared report schema requires `analysis.interval` even when too few pairs
exist to calculate an interval. A serializer-wide null-omission setting left
the field out and made the package unable to replay its own report. Override
null omission for required nullable fields, then test both a null round trip and
rejection when the field is missing.
---
category: C#
---

# A required nullable JSON field must remain serialized as explicit null

`analysis.interval` is required by the portable report schema even when the
sample is too small to produce an interval. A serializer-wide
`WhenWritingNull` option can silently omit it and make the package unable to
replay its own report. Override omission for required nullable fields and test
both a null interval round trip and malformed reports that omit the field.
