---
category: Testing & coverage
---

# Go JSON null unmarshals into a string without error, so nullable string validation needs a pointer

When allowing an empty JSON string, `json.Unmarshal` into a Go `string` cannot
also distinguish `null`: unmarshalling `null` into a non-pointer string returns
nil and leaves the zero value. A check that used to reject both null and empty
strings by testing `strings.TrimSpace(value) == ""` silently starts accepting
null when relaxed to preserve valid empty argv entries.

Decode optional JSON strings through `*string` (or validate the raw JSON kind)
and reject a nil pointer explicitly. Keep the empty string valid when the schema
allows it, and test both `""` and `null` at the same ingress boundary.
