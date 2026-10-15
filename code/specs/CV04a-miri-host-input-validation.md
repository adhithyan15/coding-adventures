# CV04a local Miri host-input validation

Specification before runner implementation. The frozen final-source script
returned101: Twig's actual host/read_byte test aborted because Miri isolation
forbids reading stdin. The earlier stale run and this complete failed run remain
excluded from acceptance. This repair concerns runner capability and input
ownership; it changes no compiler, lexer, VM instruction, assertion or test filter.

## Contract

The canonical local code/scripts/miri-twig-vm.sh must run runtime-core, dynval
and the entire Twig VM suite with --no-fail-fast as before. Keep the existing
ignore-leaks policy; introduce no UB-validation/aliasing disabling option.
Runtime-core and dynval retain host isolation. Only the Twig command needs
Miri's documented -Zmiri-disable-isolation to exercise actual host IO. Miri's
[official README](https://github.com/rust-lang/miri/blob/master/README.md)
documents this flag as permitting host access; it is not a replacement for UB
checks. Host access applies to the repository's trusted VM tests only.

Redirect that command's stdin from /dev/null, including under Git Bash on
Windows, so host/read_byte receives EOF without consuming the invoking terminal
or blocking for user input. The script must print which policy it executes.
Keep every existing test selected, including the 33554432-tick instruction cap
and the host/read_byte success/integer-result assertions. No conditional ignores,
new dependency or synthetic success paths are allowed.

## Validation

Before changing the runner, run the exact existing host_read_byte test with
controlled closed stdin under the original Miri flags: require the isolation
diagnostic and nonzero exit, rather than an assertion failure or zero selected
tests. Run the same actual test with scoped host access: require one executed
assertion test and a successful result. This is focused diagnostic evidence,
not full-suite acceptance.

After repair, shell syntax and a recording-cargo control verify all three commands,
the substrate/VM flag split, closed Twig stdin, fail-fast script status propagation
and absence of test filters. The control never substitutes for actual Miri.
Freeze the final source/runner, perform independent exact-head security review,
then run the complete canonical script and accept only terminal exit0 with
unchanged tested source. Preserve the failed source-bound log/receipt before
restarting. A harness abort remains failed even if parsed failed-test totals are0.

Re-audit and merge current main, validate changed consumers and actual production
planning before publication. CV04a acceptance still does not close CV04b,
provenance #15830 or full Closure parity.
