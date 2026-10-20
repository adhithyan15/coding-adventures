---
category: Testing & coverage
---

# A corpus-wide test that recomputes the corpus in every it() times out as content grows

tests/narration.test.ts "the whole corpus" called narrationChapters() on all
~5,800 chapters inside each of four it() blocks. Adding 745 Italian etymology
blocks pushed the first one past its 30 s budget on a CI runner (16 s locally).
The fix was not a longer timeout: narrate once at describe scope, beside the
existing loadEverything(), and let the four tests read the shared result
(tests: 1.9 s). When a test reads the whole corpus, compute it once per suite;
a per-test recomputation is a timeout that arrives with the next content PR.
