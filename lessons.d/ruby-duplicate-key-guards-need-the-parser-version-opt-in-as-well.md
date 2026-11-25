---
category: Ruby
---

# Ruby duplicate-key guards need the parser-version opt-in as well as an object class

In ruby/json 2.21.2, `object_class` does not receive duplicate member
assignments: the parser builds an ordinary hash, warns, and keeps the last
value. A duplicate-aware `Hash#[]=` therefore does not fail closed by itself.
Pass `allow_duplicate_key: false` to `JSON.parse` as well; this parser raises
before the repeated value can replace the first. The explicit option is ignored
by ruby/json 2.9.1, where the duplicate-aware object class still rejects the
input, so keep both guards and test both parser behaviors. The CI log showed
2.21.2 being fetched and the warning, but did not record the JSON version loaded
by the test process; reproduce behavior with an explicit gem activation instead
of inferring the runtime version from a download line.
