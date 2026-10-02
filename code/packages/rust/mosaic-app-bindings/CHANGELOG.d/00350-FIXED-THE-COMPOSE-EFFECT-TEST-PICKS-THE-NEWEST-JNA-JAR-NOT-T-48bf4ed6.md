### Fixed — the Compose effect test picks the newest JNA jar, not the first one found

- Test only. `tests/compose_effect_completion.rs` searched `~/.gradle` for its
  JNA, kotlinx-serialization and kotlin-stdlib jars, and took the FIRST match
  in `read_dir` order, which is arbitrary. A Gradle cache often holds several
  versions of one library: Compose brings JNA 5.19.1, while
  `kotlin-compiler-embeddable` (1.8.0 and older) brings 5.6.0, which predates Apple-silicon
  support (5.7). That is the most likely cause of the macOS arm64 failure
  "the emitted Compose host did not load the conformance runtime" seen on
  #16441; the log could not say for certain (see the debug flag below).
- The search now collects every match and takes the newest version, compared
  numerically, so 5.19 sorts above 5.6. A new unit test pins that ordering.
- The driver now runs with `-Dmosaic.app.debug=1`, so a future load failure
  prints JNA's own reason instead of only "did not load".
