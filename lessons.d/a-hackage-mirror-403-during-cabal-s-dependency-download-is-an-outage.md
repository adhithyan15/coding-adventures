---
category: CI & GitHub Actions
---

# A Hackage mirror 403 during cabal's dependency download is an outage, not a test failure

A PR that touched only the Go build tool (#18005) had a red build shard. The
failure was `haskell/activation-functions` and its dependent
`haskell/perceptron`. cabal could not fetch the test-suite dependencies
(`call-stack`, `colour`, `haskell-lexer`, `splitmix`):

```
Unexpected response 403 for
http://objects-us-east-1.dream.io/hackage-mirror/package/call-stack-0.4.0.tar.gz
```

hackage-security, cabal's secure-repository layer, takes its mirror list from
Hackage's signed `mirrors.json` and falls back to those mirrors when the
primary fetch fails. One of them returning 403 is a third-party outage, and no
test body ran.

**What to do:**
- Treat it as infra. Check that the error is a download error and not a
  compile or test error.
- Do *not* "fix" it by declaring the repository `secure: False` to bypass the
  mirror list. That drops signature verification of every package.
- Re-run CI with a base-merge push. If the 403 repeats across runs, the next
  step is a reviewed cabal-config change.
- Wait for the other shards to report before pushing, so a single re-run
  covers every failure.
