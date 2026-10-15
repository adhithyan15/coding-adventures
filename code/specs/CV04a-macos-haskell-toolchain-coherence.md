# CV04a: scope a coherent macOS Haskell compiler and SDK

Spec BEFORE tests/implementation. Published d400 job116367540126 failed in
haskell/conduit dependencies linking GHC9.4.8 against MacOSX27SDK arm64e.x1
stubs. Actual direct logSHAadcd75e554eb713cd20755f26b2ae0050ad05e47e57788c55f9ccc9e8e243cb1.
This proves a hosted link failure, not a Closure assertion or observed UB.
Do not waive the check or assume upgrading GHC alone fixes it.

Use the currently selected developer tree through /usr/bin/xcrun to discover
matching macosx clang, ld and SDK paths. Do not change xcode-select, global
DEVELOPER_DIR or SDKROOT: other Swift/iOS jobs keep their tool selection.
Clone the actual Cabal3.10 config into a fresh RUNNER_TEMP directory, preserving
repository/security/cache/store/unknown options verbatim. Discover config using
CABAL_CONFIG, CABAL_DIR/config, XDG_CONFIG_HOME/cabal/config or its default,
with documented legacy ~/.cabal fallback only if that directory exists and XDG
config does not. Require a real existing config after the setup/index steps.
Never print the config or mutate the original. Fail closed on duplicate active
managed sections or existing active managed fields instead of overwriting
reviewed options. Keep every unrelated line and option intact.

Add scoped program-locations gcc/ld and program-default-options for ghc/hsc2hs.
GHC C compile, assembly and link drivers use the selected clang; explicitly
forward the selected isysroot to those phases. hsc2hs compile/link drivers and
sysroots agree. Preserve other GHC/settings/tool flags. Quote paths as Cabal
argument tokens, reject control characters/nonabsolute macOS paths; execute
subprocess argument vectors, no shell concatenation. No dependencies added.

Before exporting CABAL_CONFIG for subsequent Haskell builds, perform actual
C compile/link/run, GHC plus foreign C compile/link/run, and offline Cabal
base-only executable+C source compile/link/run under the overlay. Every probe
requires exact expected normal output/exit. Log selected paths, tool versions,
commands and log hashes (not config contents), original/overlay config hashes
and receipt status. Publish the config path to GITHUB_ENV ONLY after all probes
pass; a failed discovery/compiler/link/runtime/Cabal step fails bootstrap and
publishes nothing. Retain logs/receipts for diagnosis; do not reduce tests.

Commit portable unittest controls before behavior: config precedence/legacy,
source preservation, exact quoted phase/sysroot options, existing field and
duplicate-section refusal, malformed paths, and no publication after any failed
probe. No mocked result counts as genuine macOS linking. Run actual Cabal
configuration parser acceptance locally using a synthetic nonsecret config if
available; GHC9.4.8/Cabal3.10 macOS hosted probe and full required build checks
remain mandatory finalhead acceptance. Add a conditional macOS+needs_haskell
bootstrap step after package-index verification. Keep other platform commands,
all required jobs, Rust source/input limits, package tests and dependencies exact.
Spec/readme/changelog/backlog/issue/checker/lessons/diff and exacthead independent
security/terminal/carrier reviews gate samePR17288 delivery. No auto until
current conflictfree/reviewclean terminal acceptable required checks, actual
fetched merge reachability before cleanup. Windows UnixFD selection repair is
separate, locally validated7e725952; native9cdca fullsix is running separately.

Primary references inspected for this design:
https://downloads.haskell.org/ghc/9.4.8/docs/users_guide/phases.html
https://cabal.readthedocs.io/en/3.10/config.html
https://cabal.readthedocs.io/en/stable/config.html
Cabal phase defaults are not arbitrary environment compiler overrides; explicit
GHC -pgm/-opt phase options route internal calls. Probe actual transitive Cabal
consumption rather than infer it from a syntactically valid config.

Miri DEFERRED/PENDING, never passed; frozen checkout/process/inputs/target/
receipts untouched/unpolled, no duplicate/termination. Finish implementation/
delivery then genuine final-source Miri. BaseESTree/ownedtypes/no crates.io
dependencies preserved. Full Closure parity/output-byte-source/transformation
lineage/objectiveOPEN; heartbeatACTIVE. Current-main/overlap audit before push.
