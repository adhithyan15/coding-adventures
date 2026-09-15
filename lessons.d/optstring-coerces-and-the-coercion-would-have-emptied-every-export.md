# `optString` coerces, and the coercion would have emptied every export

Teaching six host adapters to accept base64 as well as the legacy numeric
array, five used an explicit type test — `as? String`, `isString()`,
`is String`, `ValueKind == String`, `typeof === "string"`. The Kotlin one used
`root.optString(property, "")`, which reads like the same thing and is not:

```
  optString on an array -> "[109,112,51]"
  opt is String?        -> false
```

`org.json` **coerces**. A legacy array would have taken the base64 branch,
failed to decode, and returned `ByteArray(0)` — and the helper's contract is to
return an empty array on any mismatch, so the export would have written a
zero-byte `.apkg` and reported success.

Two things worth keeping:

- **"Get it as a string" and "is it a string" are different questions**, and
  several JSON APIs answer the first when asked the second. Five libraries made
  the distinction impossible to get wrong; one made it the default. Checking
  what the API returns for the *other* type is cheap and I nearly skipped it,
  because four working implementations felt like evidence about the fifth.
- **The failure would have been silent, which is why the check was worth
  running.** Nothing throws; the file is just empty. The same helper shape is
  in all six hosts, and it returns empty rather than raising on every
  mismatch — worth revisiting on its own.

Verified by compiling and running the org.json call rather than reasoning from
the docs, which is what turned "I think this coerces" into a fact.
