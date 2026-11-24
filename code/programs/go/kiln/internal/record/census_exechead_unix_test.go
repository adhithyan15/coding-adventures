//go:build unix

package record

// The exec-head block of the census (KILN04 D48 step 6 item 2): whether the shard
// ran with exec heads, the counts per sub-code, per supervisor unclean reason and
// per parse reason, the supervisor's kills and notifications, and the Go
// emittable count.  Counts only: no path reaches the Markdown.

import (
	"os"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

func TestCensusExecHeadBlockOnAShardWithoutExecHeads(t *testing.T) {
	f := censusDirFix(t)
	_, s, _, mdPath := runCensus(t, f)
	eh := s.ExecHead
	if eh.Sealed || eh.SupervisedSteps != 0 || eh.Kills != 0 || eh.Notifications != 0 || eh.GoEmittable != 0 {
		t.Fatalf("an unsealed shard reports exec heads: %+v", eh)
	}
	// Every closed-list key is present, zero included.
	if len(eh.SubCodes) != 13 || len(eh.Unclean) != 6 || len(eh.ParseReasons) != 5 {
		t.Fatalf("the tables are not complete: %d sub-codes, %d unclean, %d parse reasons", len(eh.SubCodes), len(eh.Unclean), len(eh.ParseReasons))
	}
	md, err := os.ReadFile(mdPath)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(md), "ran WITHOUT exec heads") {
		t.Fatalf("the Markdown does not say the shard ran without exec heads")
	}
}

func TestCensusExecHeadSealComesFromTheBinding(t *testing.T) {
	p := newPrep(t)
	p.execHead = true
	f := newStepFixOn(t, p, nil, planSteps, nil, nil, false, nil)
	acc := newCensusAcc(newRedactor(nil))
	acc.hostFrom(f.pre, "")
	if got := acc.execHeadSummary(); !got.Sealed {
		t.Fatalf("a binding sealed with exec_head reports %+v", got)
	}
}

func TestCensusExecHeadAccumulation(t *testing.T) {
	acc := newCensusAcc(newRedactor(nil))
	acc.execHead.Sealed, acc.execHead.KernelTrusted = true, true

	// The classifier's census: two sub-codes, one shed into the overflow; a kind
	// that is not a sub-code and a sub-code that is not on the list count nothing.
	acc.addCensus(&CensusDoc{
		Schema: CensusSchema, Action: "code/p/a", Overflow: map[string]int64{execHeadSubKindPrefix + "exechead-kernel": 2},
		Entries: []traced.CensusEntry{
			{Kind: execHeadSubKindPrefix + "exechead-kernel", Path: "/tmp/x/t.test", Count: 3},
			{Kind: execHeadSubKindPrefix + "exechead-pair", Path: "/tmp/x/u.test", Count: 1},
			{Kind: execHeadSubKindPrefix + "exechead-bogus", Path: "/tmp/x/v", Count: 9},
			{Kind: execHeadSubKindPrefix + "under-tmp", Path: "/tmp/x/w", Count: 4},
			{Kind: "host-read", Path: "/etc/x", Count: 5},
		},
	})
	// Step sidecars: a supervised step with a kill and a stall, a clean supervised
	// step, and an unsupervised one (its zero counts are not summed in).
	mk := func(nonce string, notified, kills int, unclean ...string) StepSidecar {
		sc := execHeadSidecar()
		if nonce != "" {
			sc.ExecHeadNonce, sc.ExecHeadSHA256, sc.ExecHeadBytes = nonce, strings.Repeat("d", 64), 117
			sc.ExecHeadNotified, sc.ExecHeadKills = notified, kills
		}
		if len(unclean) > 0 {
			sc.Clean, sc.Unclean = false, unclean
		}
		return sc
	}
	acc.sidecarFacts(mk(ehNonce, 7, 1, "exechead-stall"))
	acc.sidecarFacts(mk(ehNonce, 5, 0))
	acc.sidecarFacts(mk("", 0, 0))
	acc.explain.RecordParseReasons["exechead-digest"] = 2
	acc.execHead.GoEmittable = 4

	e := acc.execHeadSummary()
	if e.SubCodes["exechead-kernel"] != 5 || e.SubCodes["exechead-pair"] != 1 || len(e.SubCodes) != 13 {
		t.Errorf("sub-codes %v", e.SubCodes)
	}
	if e.Unclean["exechead-stall"] != 1 || e.Unclean["exechead-handover"] != 0 || len(e.Unclean) != 6 {
		t.Errorf("unclean %v", e.Unclean)
	}
	if e.ParseReasons["exechead-digest"] != 2 || len(e.ParseReasons) != 5 {
		t.Errorf("parse reasons %v", e.ParseReasons)
	}
	// The unsupervised started step disagrees with the on seal.
	if e.SealMismatch != 1 {
		t.Errorf("seal mismatch %d, want 1", e.SealMismatch)
	}
	if e.SupervisedSteps != 2 || e.Notifications != 12 || e.Kills != 1 || e.KillSteps != 1 || e.GoEmittable != 4 {
		t.Errorf("sums %+v", e)
	}

	// The Markdown carries the counts and no path.
	s := acc.summary(10)
	md, _, _ := RenderCensusMarkdown(s)
	for _, want := range []string{"ran WITH exec heads", "supervisor notifications 12", "supervisor kills 1 (in 1 steps)", "language go", "exechead-kernel` 5", "exechead-stall` 1"} {
		if !strings.Contains(md, want) {
			t.Errorf("the Markdown lacks %q", want)
		}
	}
	section := md[strings.Index(md, "## Exec heads"):]
	section = section[:strings.Index(section, "## Reasons by kind")]
	if strings.Contains(section, "/tmp") || strings.Contains(section, "t.test") {
		t.Errorf("a path reached the exec-head section:\n%s", section)
	}
}
