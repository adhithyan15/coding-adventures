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

## Review addendum before further behavior

Exactd98 securityae1901bf WITHHELD. Cabal normalizes section/field names and
accepts brace/indent variants. Case-fold managed names, preserve unrelated lines,
and fail closed on unsupported managed section syntax or structural braces;
never append a second effective override. Add executable reds for uppercase
managed fields/sections, case-normalized duplicates and brace/indent witnesses.

Cabal user-config diff/update omit program-default-options on local3.16 even
though actualGHC9.4.8 receives a deliberately invalid option and -pgmc sentinel
(offline real-build receipts22b63fd1/35b3f36f). Rendering alone is not routing
acceptance. UpstreamCabal11881/12260 additionally identifies ordering hazards,
so exact3.10 hosted routing must be demonstrated rather than inferred.
Before positive Cabal acceptance, run separate expected-refusal configurations
for absent -pgmc/-pgma/-pgml drivers. Each must fail at that exact missing tool;
a healthy default success or unrelated failure blocks config publication.
Force selectedld explicitly through clang's -fuse-ld path for real link probes.

Cabal may append hsc2hs compiler options after defaults. Generate a private
hsc2hs wrapper in the fresh output directory: safely quoted exec of the real
hsc2hs through a Python driver appending the selected cc/ld/sysroot/linker and
one probe macro AFTER caller arguments. Retain a hashed argv trace. Configure
hsc2hs-location to that wrapper. The actual Cabal probe uses Main.hsc with a
constant available only from the forced probe macro, and must record that
source's actual wrapper invocation/effective last compiler/linker arguments.
A healthy unwrapped default, missing/wrong trace, ignored sentinel, malformed
config or any actual runtime/compiler failure publishes nothing. No global
GHC, SDK, PATH, developer selection or original Cabal config mutation. Fake
traces in portable tests prove only control flow; all actual hosted probes and
full required checks remain mandatory. Update docs/lessons/backlog/issues and
obtain new exact-source security review; preserve the original WITHHELD report.
