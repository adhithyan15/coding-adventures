---
category: Testing & coverage
---

# Taking the first matching jar from a Gradle cache is nondeterministic

`mosaic-app-bindings/tests/compose_effect_completion.rs` finds its JNA jar by
walking `~/.gradle` and returning the first `jna-<digit>…jar` it meets. A Gradle
cache often holds several versions of one library: Compose brings JNA 5.19.1,
and `kotlin-compiler-embeddable` (1.8.0 and older) brings 5.6.0, which has no darwin-aarch64
support. `read_dir` order is arbitrary, so the test could compile and run against
5.6.0. On the macOS arm64 runner that is the most likely reason `Native.load`
failed with "the emitted Compose host did not load the conformance runtime".
It failed that way on #16441, which never touched the crate, after the same
lane had been green on #16391.

The host's `load()` reports JNA's reason only under `-Dmosaic.app.debug`, so the
CI log showed what failed but not why. That is why the cause above is "most
likely" rather than confirmed. The debug flag now on the driver will confirm or
refute it on the next failure.

**Fix:** collect every matching jar and take the highest version, compared
numerically. Run the driver with `-Dmosaic.app.debug=1`.

**Do instead:** a test that locates a dependency on disk must choose
deterministically (the newest, or an explicit pin), never the first in
directory order. And a probe that swallows its error needs a debug switch that
the test turns on.
