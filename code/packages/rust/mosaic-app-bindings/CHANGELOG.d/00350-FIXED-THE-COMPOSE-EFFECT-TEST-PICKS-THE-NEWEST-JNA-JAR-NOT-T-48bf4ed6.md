### Fixed — the Compose effect test picks the newest JNA jar, not the first one found

- Test only. `tests/compose_effect_completion.rs` searched `~/.gradle` for its
  JNA, kotlinx-serialization and kotlin-stdlib jars, and took the FIRST match
  in `read_dir` order, which is arbitrary. A Gradle cache often holds several
  versions of one library: Compose brings JNA 5.19.1, while
  `kotlin-compiler-embeddable` (1.8.0 and older) brings 5.6.0, which predates Apple-silicon
  support (5.7). On #16441 the macOS arm64 run failed with "the emitted Compose host did not
  load the conformance runtime". The debug flag below confirmed why: JNA's
  dispatch library was "fat file, but missing compatible architecture (have
  'i386,x86_64', need 'arm64')". In other words, a pre-5.7 JNA.
- The search now collects every match and takes the newest version, compared
  numerically, so 5.19 sorts above 5.6. JNA must also be at least 5.7.0
  (`MIN_JNA_VERSION`). When the cache holds only older JNA, as on that runner,
  the test skips with a message, as it already does when a jar is missing.
  It no longer loads a library that cannot run there. A unit test pins both
  the ordering and the minimum.
- The driver now runs with `-Dmosaic.app.debug=1`, so a future load failure
  prints JNA's own reason instead of only "did not load".
