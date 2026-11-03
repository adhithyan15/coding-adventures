# Explicit deferred XAML package experiment

Add `mosaic-compile pkg --experimental-deferred-conditionals` for reproducible
full-package native experiments. Default output stays eager. Reject the flag
for non-XAML backends before writing output. Route a default-false experiments
object through a new artifact-builder entry point while retaining existing
public builder functions unchanged as eager compatibility wrappers.

Apply the option consistently to every exported component/layout variant and
to the generated project root. Preserve package style composition, backend
tokens, runtime selection and native-complete validation. No environment switch,
source-default mutation or generated-output edit is allowed. Document that the
flag is experimental and does not promote native compatibility or performance.

Test default eager output, opt-in deferred output (including text update timing),
project and secondary/layout-variant output, and non-XAML rejection. Compile the
CLI and exercise full TaskApp package generation with the retained runtime, then
build/run the generated Release host with repeated-draft lifecycle acceptance.

## Validation (2026-11-03)

Full artifact-builder and compiler test suites passed, including 348 builder
unit tests and 16 compiler unit tests plus integration/doc tests. The new
fixture checks two exports and both default/touch layouts with and without a
project, using default eager and explicit deferred settings. A real CLI call
rejected HTML plus the experiment without creating output.

The real package CLI generated TaskApp with the flag, native-complete and the
retained runtime DLL. Its Release x64 host built and passed the 144-DPI smoke:
three edited draft Board/List cycles, scheduling/task lifecycle, and persisted
restart. No emitter default or generated source was edited. This is native host
acceptance with a retained runtime, not a fresh downloaded release. Security
review and git diff --check passed.
