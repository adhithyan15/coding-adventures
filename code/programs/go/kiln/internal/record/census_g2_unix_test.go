//go:build unix

package record

// G2 is a reported target, never a gate (KILN04 D49).

import (
	"strings"
	"testing"
)

// The target block carries the share p50, whether it is met, the image and the
// parse throughput; the Markdown says "not a gate".
func TestCensusG2IsReportedNotGated(t *testing.T) {
	a := newCensusAcc(nil)
	a.host.ImageID = "ubuntu-24.04"
	a.samples = []parseSample{
		{size: 8_000_000, ms: 626, share: 0.107}, {size: 8_000_000, ms: 700, share: 0.12},
		{size: 8_000_000, ms: 650, share: 0.11},
	}
	a.shares = []float64{0.107, 0.12, 0.11}
	s := a.summary(10)
	g := s.Timing.G2Target
	if g.Met || g.ShareN != 3 || g.TargetShareP50 != 0.10 || g.ShareP50 != 0.11 || g.ImageID != "ubuntu-24.04" || g.ParseMBPerSP50 <= 0 {
		t.Errorf("g2 target: %+v", g)
	}
	md, _, _ := RenderCensusMarkdown(s)
	if !strings.Contains(md, "G2 target (reported, not a gate)") || !strings.Contains(md, "not met") || !strings.Contains(md, "ubuntu-24.04") {
		t.Errorf("markdown lacks the reported target:\n%s", md)
	}

	// Under the target: met. No shares at all: not met, and ShareN says why.
	b := newCensusAcc(nil)
	b.shares = []float64{0.05, 0.06}
	if !b.summary(10).Timing.G2Target.Met {
		t.Errorf("share p50 under 10%% should meet the target")
	}
	if g := newCensusAcc(nil).summary(10).Timing.G2Target; g.Met || g.ShareN != 0 {
		t.Errorf("no shares: %+v", g)
	}
}
