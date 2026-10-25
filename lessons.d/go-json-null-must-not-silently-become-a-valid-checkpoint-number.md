---
category: Security boundaries
---

# Go JSON null must not silently become a valid checkpoint number

The standard Go decoder accepts JSON `null` for scalar numeric destinations
without reporting a type error, leaving the destination's zero value. Missing
fields similarly acquire zero values. For a portable neural checkpoint, zero
is a valid weight and step, so a successful struct decode alone is not enough.

Validate exact required object keys and non-null fields before typed decoding;
check every weight/bias token is a numeric value, not null, before admitting
the array. Then validate finite arithmetic, shapes and safe step bounds at the
owned model boundary. Test literal null numeric cells separately from empty,
ragged and missing-field cases. Keep malformed input fixtures as data rather
than executable mutation functions.
