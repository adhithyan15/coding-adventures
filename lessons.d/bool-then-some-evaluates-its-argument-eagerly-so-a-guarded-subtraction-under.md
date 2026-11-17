---
category: Rust
---

# bool::then_some evaluates its argument eagerly, so a guarded subtraction under it still underflows

`(n >= 2 && n >= k && cs[n - k..] == *posts).then_some(n - k)` looks guarded, but
`then_some(n - k)` computes `n - k` before it knows the condition, so `n < k` panics with
"attempt to subtract with overflow" in a debug build. The WX-G post-state slot resolver had
exactly this; the code reviewed fine and every hand-written case passed. A brute-force
differential test over every result shape of depth 2 (`tests/wxg_slots.rs`) found it on its
first run.

Use `.then(|| n - k)` whenever the argument can fail, and differential-test small resolvers
against an enumerator that is written differently rather than against hand-picked cases.
