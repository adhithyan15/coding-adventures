---
category: CI & GitHub Actions
---

# Verify document shard identities with the actual reconstruction gate

PR #17288 passed lesson validation and native Rust tests, but its required
Repo-wide metadata contracts job failed before metadata tests because the
Mermaid changelog shard omitted the eight-hex heading digest. Lesson validation
does not parse changelog shards. Preserve the actual failing job log, derive
the filename from the exact heading with docSlug and headingDigest, then run
the real check:doc-shards reconstruction command before publication.

On Windows the npm lifecycle shell could not find node despite PowerShell
resolving it. The local repair installed dependencies without lifecycle scripts,
then explicitly executed the unchanged local-deps.mjs with npm_execpath, the
TypeScript compiler and doc-shard-cli.js --check using the bundled Node binary.
Do not treat failed setup attempts as executed contract checks. All three real
package lifecycle/build/check commands still run; no CI command changes.
