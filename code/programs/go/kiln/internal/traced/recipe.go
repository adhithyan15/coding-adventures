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
//	the number of traces differs from the number of lines
//	a trace has no step exec, or its argv could not be decoded exactly
//	an argv is not exactly [<shell>, "-c", <line>] with <line> as expected
//
// argv[0] is not compared: a shell may be started as "sh" or "/bin/sh", and
// WHICH program ran is covered by the exec observation of the step itself.
func CheckRecipe(steps []*trace.Trace, lines []string) []string {
	bad := []string{rRecipe}
	if len(steps) != len(lines) {
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
	return nil
}
