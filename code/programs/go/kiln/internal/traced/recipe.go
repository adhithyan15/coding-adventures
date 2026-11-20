package traced

import "github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"

// CheckRecipe is the recipe check of KILN04 section 4.3.
//
// The old build tool reads BUILD files from the writable disk, outside any
// trace, so an earlier BUILD could rewrite a later package's steps and have
// the rewritten steps recorded under the original recipe's key.  The defence
// is to compare what each step REALLY ran with what the weak key's recipe says
// it should run: the step's exec was `sh -c <line>`, and <line>, byte for byte,
// must be the matching line of the BUILD variant that came from Source.
//
// lines is the expected `sh -c` line of each step, in order.  The traces are
// the action's steps, in order.  The result is nil when everything matches,
// else {"trace-incomplete:recipe-mismatch"}:
//
//	there are MORE traces than lines
//	there are FEWER traces than lines and they are not a failed prefix (below)
//	a trace has no step exec, or its argv could not be decoded exactly
//	an argv is not exactly [<shell>, "-c", <line>] with <line> as expected
//
// argv[0] is not compared: a shell may be started as "sh" or "/bin/sh", and
// WHICH program ran is covered by the exec observation of the step itself.
//
// The failed prefix (KILN04 section 4.3, D36).  The build tool runs an action's
// steps one after another and stops at the first that fails
// (build-tool/internal/executor/executor.go: the loop at line 474 returns at
// line 531, "failed", on the first step error), so an action that failed has
// FEWER traces than recipe lines, and that is not a mismatch: the later steps
// never ran, so they wrote nothing.  Fewer traces are accepted exactly when
//
//	every trace matches its line, byte for byte, as above (the traces are a
//	prefix of the recipe, in order),
//	the traces before the last exited with status 0 (the executor ran a later
//	step only because they succeeded), and
//	the LAST trace's step exited on its own with a non-zero status: it has an
//	exit line, it was not killed by a signal, and the code is not 0.
//
// Then the verdict is the one the step statuses already give: stepStatus raised
// `failed` for the last step, and the write set stays whatever the traces say
// (a killed step, a core dump or a missing exit line still make it incomplete
// there).  Nothing here raises a reason.
//
// Why this is safe.  A BUILD rewritten to drop trailing steps ends on a step
// that SUCCEEDED, so it is still a mismatch.  To pass, a rewritten BUILD would
// have to end on a failing step, and an action that failed never emits a
// manifest, so no cache entry is produced from the claim.  The writes of the
// steps that did run are fully traced: nothing runs after the failure.
// Everything else keeps the old verdict: a prefix whose last step succeeded
// (the action stopped for a reason the traces do not show: the step could not
// be started, a descendant held the output pipe, the run was interrupted), a
// step that differs from its line, a reordered recipe, more traces than lines.
//
// An EMPTY trace set beside a non-empty recipe is a mismatch: no step ran, so
// there is no failed step to excuse the missing ones (and Classify refuses an
// empty trace set before it gets here).
func CheckRecipe(steps []*trace.Trace, lines []string) []string {
	bad := []string{rRecipe}
	if len(steps) > len(lines) {
		return bad
	}
	for i, t := range steps {
		if t == nil {
			return bad
		}
		ev := t.StepEvent()
		if ev == nil || len(ev.Argv) != 3 || ev.Argv[1] != "-c" || ev.Argv[2] != lines[i] {
			return bad
		}
	}
	if len(steps) == len(lines) {
		return nil
	}
	if failedPrefix(steps) {
		return nil
	}
	return bad
}

// failedPrefix reports whether steps, already known to match the first
// len(steps) recipe lines, end on a step that exited non-zero after steps that
// all exited zero.
func failedPrefix(steps []*trace.Trace) bool {
	if len(steps) == 0 {
		return false
	}
	for i, t := range steps {
		ex := t.StepExit()
		if ex == nil || ex.Killed {
			return false
		}
		last := i == len(steps)-1
		if last == (ex.Code == 0) {
			// The last step must have failed, an earlier one must not have.
			return false
		}
	}
	return true
}
