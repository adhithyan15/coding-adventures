---
category: CI & GitHub Actions
---

# Freeze Flutter pub cache keys before generated build trees make recursive globbing unbounded

The macOS build at b3abbdefa44c0ea77b3a2b6ec8a481eaa7bfbb44 completed
535 packages with zero failures (4,790 skipped; 5,325 total). The job then failed
in `Post Set up Flutter`: the pinned action evaluated
`hashFiles('**/pubspec.lock')` over the generated checkout and exceeded its
120-second expression limit. A package success and a successful CI job are
different receipts.

The action appends that hash unconditionally to its pub cache key. Supplying a
custom `pub-cache-key` alone does not remove the recursive traversal. Keep the
Flutter SDK cache, but explicitly disable the action's optional pub cache with
`pub-cache: false`. Dart pub still resolves and fetches dependencies normally;
no build, runtime, coverage, assertion or deadline is weakened.

If pub caching is introduced again, compute an identity from bounded tracked
source inputs before generating build trees, and give the cache action that
already computed output. Do not put recursive repository globbing in a post-job
cache expression. Verify a terminal successful hosted job, including cleanup.
