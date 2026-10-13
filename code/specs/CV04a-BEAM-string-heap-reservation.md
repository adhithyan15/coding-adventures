# CV04a BEAM string heap reservation

Hosted Linux run38323457119 published822 fails sparse_real_and_string_value_copies_run_on_beam
b11_str_3d with erl_gc.c775 Overrun stack and heap. Actual WindowsOTP29 sparse
native case and300repeat forced-GC controls at5b pass; they do not clear Linux failure.
Pre-repair compiled byte receipt frozen. No installed WSL; no Linux acceptance.

Inspect iir-to-beam str_const: raw put_list constructs2words per ASCII byte without
test_heap. Unlike a call, this instruction does not reserve heap itself. Add a
bounded authored long-string native control to force construction beyond initial
heap, and an exact lowering regression for heap reservation before the first
construction write. Spec/test/red evidence precedes repair. Existing typedASCII
validation/string semantics, all prior BEAM11 sparse six cases and array/frame/
import liveness tests remain. No skips, assertion weakening or new dependencies.

Candidate repair reserves exactly2*byte_length words using OP_TEST_HEAP, before
list construction, with the existing conservative normal-X live count. Prologue
initializes all future normal X registers to nil after frame allocation; existing
call-restoration sanitizes clobbered nonlive registers. Scratch registers are not
roots for this expansion. Check overflow of byte-to-word count; do not add a
blanket heap limit or change string byte order/nil representation. Empty strings
need no heap. Verify long constants remain correct when other strings/scalars
are live and after real imported calls. If actual red contradicts this cause,
retain evidence and revise spec; never claim hosted defect fixed from proxy green.

Affected metadata cone/fulltests/native Erlang and strictlint, exact-final
independent security, currenthead Linux CI including original b11_str_3d required.
Review usage limit is not PASS. Main9b47 powered/ordering imports still pending.
Miri DEFERRED/PENDING, frozen run/497inputs/source/receipts/target untouched/unpolled;
full compiler parity/output joins/alllineage/fullgoalOPEN, same solePR17288.

Actual red62ff79f: exact lowering test fails (0reservations vs1); authored2048-byte
native source makes real Erlang exit nonzero (no stderr), both Cargo101/0pass1fail.
Existing six-case sparse test and300forced-GC original3d repeats passed on local
OTP29; retained, never substituted for hosted Linux failure. Canonical runtime
GC_TEST/test_heap semantics: https://www.erlang.org/docs/29/apps/erts/beam_makeops.html#the-macros-tab-file
