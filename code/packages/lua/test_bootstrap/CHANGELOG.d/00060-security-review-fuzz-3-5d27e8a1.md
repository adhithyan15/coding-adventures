## 0.2.3

- The nested-coroutine cap counts coroutines alive at once: a full list is compacted (dead
  coroutines' hooks are checked, then dropped), so thousands of sequential generators are a pass.
  Over 4,096 alive at once is a `too-many-coroutines` crash, never a timeout, never a leak.
- Spec 5.3 now says nested coroutines are held strongly, with compaction; README documents the cap.
- Tests: 10,000 sequential generators, 4,097 suspended, a dead tamperer dropped by compaction;
  the vacuous `Limits.HOOK_COUNT` write is gone from the huge-count test.
