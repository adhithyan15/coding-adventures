//go:build unix

package record

// Review follow-ups of D48 step 6: the census's own seal-mismatch count, bounded
// and saturating counters, and the sub-code list held equal to the classifier's.

import (
	"slices"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

func supervisedSidecar() StepSidecar {
	sc := execHeadSidecar()
	sc.ExecHeadNonce, sc.ExecHeadSHA256, sc.ExecHeadBytes = ehNonce, strings.Repeat("d", 64), 117
	return sc
}

// The census's own count of what collect voids for: both directions, and a step
// that never started is not a disagreement.
func TestCensusExecHeadSealMismatchCountsBothDirections(t *testing.T) {
	for _, c := range []struct {
		name   string
		sealed bool
		sc     StepSidecar
		want   int
	}{
		{"on seal, supervised", true, supervisedSidecar(), 0},
		{"on seal, bare", true, execHeadSidecar(), 1},
		{"off seal, bare", false, execHeadSidecar(), 0},
		{"off seal, supervised", false, supervisedSidecar(), 1},
	} {
		acc := newCensusAcc(newRedactor(nil))
		acc.execHead.Sealed = c.sealed
		acc.sidecarFacts(c.sc)
		if got := acc.execHeadSummary().SealMismatch; got != c.want {
			t.Errorf("%s: %d, want %d", c.name, got, c.want)
		}
	}
	acc := newCensusAcc(newRedactor(nil))
	acc.execHead.Sealed = true
	sc := execHeadSidecar()
	sc.Started, sc.StepNetNS = false, false
	acc.sidecarFacts(sc)
	if acc.execHeadSummary().SealMismatch != 0 {
		t.Error("an unstarted step counted")
	}
}

func TestCensusExecHeadSumsSaturate(t *testing.T) {
	if got := satAdd(1<<62-1, 5); got != 1<<62 {
		t.Fatalf("satAdd near the top: %d", got)
	}
	if got := satAdd(1<<62, 1<<62); got != 1<<62 {
		t.Fatalf("satAdd at the top: %d", got)
	}
	if satAdd(2, 3) != 5 {
		t.Fatal("satAdd plain")
	}
	acc := newCensusAcc(newRedactor(nil))
	sc := supervisedSidecar()
	sc.ExecHeadNotified, sc.ExecHeadKills = MaxExecHeadCount, MaxExecHeadCount
	for range 4 {
		acc.sidecarFacts(sc)
	}
	if e := acc.execHeadSummary(); e.Notifications != 4*MaxExecHeadCount || e.Kills != 4*MaxExecHeadCount {
		t.Fatalf("sums %+v", e)
	}
}

func TestStepSidecarExecHeadCountsAreCapped(t *testing.T) {
	sc := supervisedSidecar()
	sc.ExecHeadNotified, sc.ExecHeadKills = MaxExecHeadCount, MaxExecHeadCount
	if _, err := EncodeStepSidecar(sc); err != nil {
		t.Fatalf("a count at the cap: %v", err)
	}
	for _, mut := range []func(*StepSidecar){
		func(s *StepSidecar) { s.ExecHeadNotified = MaxExecHeadCount + 1 },
		func(s *StepSidecar) { s.ExecHeadKills = MaxExecHeadCount + 1 },
	} {
		s := sc
		mut(&s)
		if _, err := EncodeStepSidecar(s); err == nil {
			t.Error("a count over the cap encoded")
		}
	}
}

// The census keeps its own copy of the exec-head sub-codes (to always print all
// of them); a code the classifier gains must not be dropped from it silently.
func TestCensusExecHeadSubCodesEqualTheClassifiers(t *testing.T) {
	got, want := slices.Clone(execHeadSubCodes), traced.ExecHeadSubCodes()
	slices.Sort(got)
	slices.Sort(want)
	if len(want) == 0 || !slices.Equal(got, want) {
		t.Fatalf("the census's sub-codes %v differ from the classifier's %v", got, want)
	}
}
