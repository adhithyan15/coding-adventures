### Perl `blib` generated-directory registry repair

- Added exact lowercase `blib` to the language-neutral generated-directory
  registry while retaining authored `Blib` and `blib-example` directories.
- Extended both neutral source-collection fixtures and propagated the governed
  registry and boundary digests through every checked build-tool projection.
- Added direct Perl evidence and refreshed the Python, Ruby, Rust, TypeScript,
  Go, Haskell, Swift, C#, and F# consumers of the shared authority.
- Prime and verify Cabal's Hackage index cache after Haskell setup so fresh
  hosted runners do not hand the first package test a test-disabled plan.
- Keep Haskell package and build-tool project test components enabled during
  dependency solving so each BUILD front can execute its declared test suite.
