---
category: Perl
---

# JSON null comes back as `JsonValue::Null` blessed object, not `undef`

Use `JsonSerializer::is_null($v)` to normalize. Tests asserting `$v == undef` fail.
