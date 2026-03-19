package branchpredictor

import (
	"math"
	"testing"
)

// ─── Test Helpers ──────────────────────────────────────────────────────────────

func assertClose(t *testing.T, got, want, epsilon float64, msg string) {
	t.Helper()
	if math.Abs(got-want) > epsilon {
		t.Errorf("%s: got %.4f, want %.4f (epsilon %.4f)", msg, got, want, epsilon)
	}
}

// ─── PredictionStats Tests ─────────────────────────────────────────────────────

func TestPredictionStatsInitial(t *testing.T) {
	s := &PredictionStats{}
	if s.Predictions != 0 || s.Correct != 0 || s.Incorrect != 0 {
		t.Error("expected zero initial state")
	}
	assertClose(t, s.Accuracy(), 0.0, 0.001, "initial accuracy")
	assertClose(t, s.MispredictionRate(), 0.0, 0.001, "initial misprediction rate")
}

func TestPredictionStatsRecordCorrect(t *testing.T) {
	s := &PredictionStats{}
	s.Record(true)
	if s.Predictions != 1 || s.Correct != 1 || s.Incorrect != 0 {
		t.Errorf("expected 1/1/0, got %d/%d/%d", s.Predictions, s.Correct, s.Incorrect)
	}
	assertClose(t, s.Accuracy(), 100.0, 0.001, "accuracy after 1 correct")
	assertClose(t, s.MispredictionRate(), 0.0, 0.001, "misprediction rate after 1 correct")
}

func TestPredictionStatsRecordIncorrect(t *testing.T) {
	s := &PredictionStats{}
	s.Record(false)
	if s.Predictions != 1 || s.Correct != 0 || s.Incorrect != 1 {
		t.Errorf("expected 1/0/1, got %d/%d/%d", s.Predictions, s.Correct, s.Incorrect)
	}
	assertClose(t, s.Accuracy(), 0.0, 0.001, "accuracy after 1 incorrect")
	assertClose(t, s.MispredictionRate(), 100.0, 0.001, "misprediction rate after 1 incorrect")
}

func TestPredictionStatsMixed(t *testing.T) {
	s := &PredictionStats{}
	s.Record(true)
	s.Record(true)
	s.Record(false)
	assertClose(t, s.Accuracy(), 66.67, 0.01, "mixed accuracy")
	assertClose(t, s.MispredictionRate(), 33.33, 0.01, "mixed misprediction rate")
}

func TestPredictionStatsReset(t *testing.T) {
	s := &PredictionStats{}
	s.Record(true)
	s.Record(false)
	s.Reset()
	if s.Predictions != 0 || s.Correct != 0 || s.Incorrect != 0 {
		t.Error("expected zero state after reset")
	}
	assertClose(t, s.Accuracy(), 0.0, 0.001, "accuracy after reset")
}

// ─── AlwaysTakenPredictor Tests ────────────────────────────────────────────────

func TestAlwaysTakenPredictsTrue(t *testing.T) {
	p := NewAlwaysTakenPredictor()
	pred := p.Predict(0x100)
	if !pred.Taken {
		t.Error("expected taken=true")
	}
	assertClose(t, pred.Confidence, 0.0, 0.001, "confidence")
}

func TestAlwaysTakenCorrectOnTaken(t *testing.T) {
	p := NewAlwaysTakenPredictor()
	p.Update(0x100, true, NoTarget)
	if p.Stats().Correct != 1 || p.Stats().Incorrect != 0 {
		t.Error("expected 1 correct")
	}
}

func TestAlwaysTakenIncorrectOnNotTaken(t *testing.T) {
	p := NewAlwaysTakenPredictor()
	p.Update(0x100, false, NoTarget)
	if p.Stats().Correct != 0 || p.Stats().Incorrect != 1 {
		t.Error("expected 1 incorrect")
	}
}

func TestAlwaysTakenLoopAccuracy(t *testing.T) {
	p := NewAlwaysTakenPredictor()
	for i := 0; i < 9; i++ {
		p.Update(0x100, true, NoTarget)
	}
	p.Update(0x100, false, NoTarget)
	assertClose(t, p.Stats().Accuracy(), 90.0, 0.01, "loop accuracy")
}

func TestAlwaysTakenReset(t *testing.T) {
	p := NewAlwaysTakenPredictor()
	p.Update(0x100, true, NoTarget)
	p.Reset()
	if p.Stats().Predictions != 0 {
		t.Error("expected zero predictions after reset")
	}
}

func TestAlwaysTakenDifferentPCs(t *testing.T) {
	p := NewAlwaysTakenPredictor()
	if !p.Predict(0x100).Taken || !p.Predict(0x200).Taken {
		t.Error("expected taken for all PCs")
	}
}

// ─── AlwaysNotTakenPredictor Tests ─────────────────────────────────────────────

func TestAlwaysNotTakenPredictsFalse(t *testing.T) {
	p := NewAlwaysNotTakenPredictor()
	pred := p.Predict(0x100)
	if pred.Taken {
		t.Error("expected taken=false")
	}
}

func TestAlwaysNotTakenCorrectOnNotTaken(t *testing.T) {
	p := NewAlwaysNotTakenPredictor()
	p.Update(0x100, false, NoTarget)
	if p.Stats().Correct != 1 {
		t.Error("expected 1 correct")
	}
}

func TestAlwaysNotTakenIncorrectOnTaken(t *testing.T) {
	p := NewAlwaysNotTakenPredictor()
	p.Update(0x100, true, NoTarget)
	if p.Stats().Incorrect != 1 {
		t.Error("expected 1 incorrect")
	}
}

func TestAlwaysNotTakenMixedAccuracy(t *testing.T) {
	p := NewAlwaysNotTakenPredictor()
	for i := 0; i < 5; i++ {
		p.Update(0x100, true, NoTarget)
	}
	for i := 0; i < 5; i++ {
		p.Update(0x100, false, NoTarget)
	}
	assertClose(t, p.Stats().Accuracy(), 50.0, 0.01, "mixed accuracy")
}

func TestAlwaysNotTakenReset(t *testing.T) {
	p := NewAlwaysNotTakenPredictor()
	p.Update(0x100, false, NoTarget)
	p.Reset()
	if p.Stats().Predictions != 0 {
		t.Error("expected zero predictions after reset")
	}
}

// ─── BTFNTPredictor Tests ──────────────────────────────────────────────────────

func TestBTFNTColdStartPredictsNotTaken(t *testing.T) {
	p := NewBTFNTPredictor()
	pred := p.Predict(0x108)
	if pred.Taken {
		t.Error("expected not-taken on cold start")
	}
	assertClose(t, pred.Confidence, 0.0, 0.001, "cold start confidence")
}

func TestBTFNTBackwardBranch(t *testing.T) {
	p := NewBTFNTPredictor()
	p.Update(0x108, true, 0x100) // teach backward branch
	pred := p.Predict(0x108)
	if !pred.Taken {
		t.Error("expected taken for backward branch")
	}
	assertClose(t, pred.Confidence, 0.5, 0.001, "backward confidence")
	if pred.Target != 0x100 {
		t.Errorf("expected target 0x100, got 0x%x", pred.Target)
	}
}

func TestBTFNTForwardBranch(t *testing.T) {
	p := NewBTFNTPredictor()
	p.Update(0x200, false, 0x20C) // teach forward branch
	pred := p.Predict(0x200)
	if pred.Taken {
		t.Error("expected not-taken for forward branch")
	}
}

func TestBTFNTEqualTarget(t *testing.T) {
	p := NewBTFNTPredictor()
	p.Update(0x100, true, 0x100) // degenerate: target == pc
	pred := p.Predict(0x100)
	if !pred.Taken {
		t.Error("expected taken for equal target")
	}
}

func TestBTFNTLoopAccuracy(t *testing.T) {
	p := NewBTFNTPredictor()
	// First encounter -- target is stored before checking prediction,
	// so even the first update knows it's a backward branch -> predicts taken
	p.Update(0x108, true, 0x100)
	// Subsequent iterations -- backward branch predicts taken, correct
	for i := 0; i < 8; i++ {
		p.Update(0x108, true, 0x100)
	}
	// Loop exit -- backward still predicts taken, wrong
	p.Update(0x108, false, 0x100)
	// 9 correct + 1 exit mispredict = 90%
	assertClose(t, p.Stats().Accuracy(), 90.0, 0.01, "loop accuracy")
}

func TestBTFNTReset(t *testing.T) {
	p := NewBTFNTPredictor()
	p.Update(0x108, true, 0x100)
	p.Reset()
	if p.Stats().Predictions != 0 {
		t.Error("expected zero predictions after reset")
	}
	pred := p.Predict(0x108)
	if pred.Taken {
		t.Error("expected not-taken after reset (cold start)")
	}
}

func TestBTFNTNoTargetDoesNotOverwrite(t *testing.T) {
	p := NewBTFNTPredictor()
	p.Update(0x108, true, 0x100)
	p.Update(0x108, true, NoTarget)
	pred := p.Predict(0x108)
	if !pred.Taken || pred.Target != 0x100 {
		t.Error("NoTarget update should not overwrite existing target")
	}
}

// ─── OneBitPredictor Tests ─────────────────────────────────────────────────────

func TestOneBitColdStart(t *testing.T) {
	p := NewOneBitPredictor(1024)
	pred := p.Predict(0x100)
	if pred.Taken {
		t.Error("expected not-taken on cold start")
	}
	assertClose(t, pred.Confidence, 0.5, 0.001, "cold start confidence")
}

func TestOneBitLearnsTaken(t *testing.T) {
	p := NewOneBitPredictor(1024)
	p.Update(0x100, true, NoTarget)
	pred := p.Predict(0x100)
	if !pred.Taken {
		t.Error("expected taken after learning taken")
	}
}

func TestOneBitLearnsNotTaken(t *testing.T) {
	p := NewOneBitPredictor(1024)
	p.Update(0x100, true, NoTarget)
	p.Update(0x100, false, NoTarget)
	pred := p.Predict(0x100)
	if pred.Taken {
		t.Error("expected not-taken after learning not-taken")
	}
}

func TestOneBitDoubleMisprediction(t *testing.T) {
	p := NewOneBitPredictor(1024)
	// Simulate 10-iteration loop
	p.Update(0x100, true, NoTarget) // wrong (cold)
	for i := 0; i < 8; i++ {
		p.Update(0x100, true, NoTarget) // correct
	}
	p.Update(0x100, false, NoTarget) // wrong (exit)
	// 8 correct out of 10 = 80%
	assertClose(t, p.Stats().Accuracy(), 80.0, 0.01, "loop accuracy")
}

func TestOneBitAliasing(t *testing.T) {
	p := NewOneBitPredictor(4)
	// Branch at 0x04 -> index 0, set to taken
	p.Update(0x04, true, NoTarget)
	// Branch at 0x00 -> also index 0, overwrites with not-taken
	p.Update(0x00, false, NoTarget)
	// Now predicting for 0x04 reads the corrupted entry
	pred := p.Predict(0x04)
	if pred.Taken {
		t.Error("expected not-taken due to aliasing corruption")
	}
}

func TestOneBitReset(t *testing.T) {
	p := NewOneBitPredictor(1024)
	p.Update(0x100, true, NoTarget)
	p.Reset()
	if p.Stats().Predictions != 0 {
		t.Error("expected zero predictions after reset")
	}
	pred := p.Predict(0x100)
	if pred.Taken {
		t.Error("expected not-taken after reset")
	}
}

func TestOneBitDifferentBranches(t *testing.T) {
	p := NewOneBitPredictor(1024)
	p.Update(0x01, true, NoTarget)
	p.Update(0x02, false, NoTarget)
	if !p.Predict(0x01).Taken {
		t.Error("branch 0x01 should be taken")
	}
	if p.Predict(0x02).Taken {
		t.Error("branch 0x02 should be not-taken")
	}
}

// ─── TwoBitState Tests ─────────────────────────────────────────────────────────

func TestTwoBitStateTakenOutcome(t *testing.T) {
	tests := []struct {
		input, want TwoBitState
	}{
		{StronglyNotTaken, WeaklyNotTaken},
		{WeaklyNotTaken, WeaklyTaken},
		{WeaklyTaken, StronglyTaken},
		{StronglyTaken, StronglyTaken}, // saturates
	}
	for _, tt := range tests {
		got := tt.input.TakenOutcome()
		if got != tt.want {
			t.Errorf("TakenOutcome(%d): got %d, want %d", tt.input, got, tt.want)
		}
	}
}

func TestTwoBitStateNotTakenOutcome(t *testing.T) {
	tests := []struct {
		input, want TwoBitState
	}{
		{StronglyTaken, WeaklyTaken},
		{WeaklyTaken, WeaklyNotTaken},
		{WeaklyNotTaken, StronglyNotTaken},
		{StronglyNotTaken, StronglyNotTaken}, // saturates
	}
	for _, tt := range tests {
		got := tt.input.NotTakenOutcome()
		if got != tt.want {
			t.Errorf("NotTakenOutcome(%d): got %d, want %d", tt.input, got, tt.want)
		}
	}
}

func TestTwoBitStatePredictsTaken(t *testing.T) {
	if StronglyNotTaken.PredictsTaken() {
		t.Error("SNT should not predict taken")
	}
	if WeaklyNotTaken.PredictsTaken() {
		t.Error("WNT should not predict taken")
	}
	if !WeaklyTaken.PredictsTaken() {
		t.Error("WT should predict taken")
	}
	if !StronglyTaken.PredictsTaken() {
		t.Error("ST should predict taken")
	}
}

// ─── TwoBitPredictor Tests ─────────────────────────────────────────────────────

func TestTwoBitInitialPredictsNotTaken(t *testing.T) {
	p := NewTwoBitPredictor(1024, WeaklyNotTaken)
	pred := p.Predict(0x100)
	if pred.Taken {
		t.Error("expected not-taken initially")
	}
	assertClose(t, pred.Confidence, 0.5, 0.001, "initial confidence")
}

func TestTwoBitOneTakenFlips(t *testing.T) {
	p := NewTwoBitPredictor(1024, WeaklyNotTaken)
	p.Update(0x100, true, NoTarget)
	pred := p.Predict(0x100)
	if !pred.Taken {
		t.Error("expected taken after one taken")
	}
	assertClose(t, pred.Confidence, 0.5, 0.001, "weak taken confidence")
}

func TestTwoBitTwoTakenStronglyTaken(t *testing.T) {
	p := NewTwoBitPredictor(1024, WeaklyNotTaken)
	p.Update(0x100, true, NoTarget)
	p.Update(0x100, true, NoTarget)
	pred := p.Predict(0x100)
	if !pred.Taken {
		t.Error("expected taken")
	}
	assertClose(t, pred.Confidence, 1.0, 0.001, "strongly taken confidence")
}

func TestTwoBitHysteresis(t *testing.T) {
	p := NewTwoBitPredictor(1024, WeaklyNotTaken)
	// Get to StronglyTaken
	for i := 0; i < 3; i++ {
		p.Update(0x100, true, NoTarget)
	}
	if p.GetState(0x100) != StronglyTaken {
		t.Error("expected StronglyTaken")
	}
	// One not-taken only goes to WeaklyTaken
	p.Update(0x100, false, NoTarget)
	if p.GetState(0x100) != WeaklyTaken {
		t.Error("expected WeaklyTaken after one not-taken")
	}
	pred := p.Predict(0x100)
	if !pred.Taken {
		t.Error("WeaklyTaken should still predict taken")
	}
}

func TestTwoBitTwoNotTakenFlips(t *testing.T) {
	p := NewTwoBitPredictor(1024, WeaklyNotTaken)
	p.Update(0x100, true, NoTarget)  // WNT -> WT
	p.Update(0x100, false, NoTarget) // WT -> WNT
	p.Update(0x100, false, NoTarget) // WNT -> SNT
	pred := p.Predict(0x100)
	if pred.Taken {
		t.Error("expected not-taken after two not-taken")
	}
	assertClose(t, pred.Confidence, 1.0, 0.001, "strongly not-taken confidence")
}

func TestTwoBitCustomInitialState(t *testing.T) {
	p := NewTwoBitPredictor(256, StronglyTaken)
	pred := p.Predict(0x100)
	if !pred.Taken {
		t.Error("expected taken with StronglyTaken initial state")
	}
	assertClose(t, pred.Confidence, 1.0, 0.001, "strongly taken confidence")
}

func TestTwoBitLoopSolvesDoubleMispredict(t *testing.T) {
	p := NewTwoBitPredictor(1024, WeaklyNotTaken)
	// Simulate 10-iteration loop + 1 re-entry
	for i := 0; i < 9; i++ {
		p.Update(0x100, true, NoTarget)
	}
	p.Update(0x100, false, NoTarget)
	p.Update(0x100, true, NoTarget)
	// 1 wrong + 8 correct + 1 wrong + 1 correct = 9/11
	assertClose(t, p.Stats().Accuracy(), 81.82, 0.01, "loop accuracy")
}

func TestTwoBitReset(t *testing.T) {
	p := NewTwoBitPredictor(1024, WeaklyNotTaken)
	p.Update(0x100, true, NoTarget)
	p.Reset()
	if p.Stats().Predictions != 0 {
		t.Error("expected zero predictions after reset")
	}
	pred := p.Predict(0x100)
	if pred.Taken {
		t.Error("expected not-taken after reset")
	}
}

func TestTwoBitGetState(t *testing.T) {
	p := NewTwoBitPredictor(1024, WeaklyNotTaken)
	if p.GetState(0x100) != WeaklyNotTaken {
		t.Error("expected WeaklyNotTaken initially")
	}
	p.Update(0x100, true, NoTarget)
	if p.GetState(0x100) != WeaklyTaken {
		t.Error("expected WeaklyTaken after one taken")
	}
}

// ─── BranchTargetBuffer Tests ──────────────────────────────────────────────────

func TestBTBColdLookupMisses(t *testing.T) {
	b := NewBranchTargetBuffer(256)
	result := b.Lookup(0x01)
	if result != NoTarget {
		t.Errorf("expected NoTarget, got %d", result)
	}
	if b.Lookups != 1 || b.Hits != 0 || b.Misses != 1 {
		t.Error("expected 1 lookup, 0 hits, 1 miss")
	}
}

func TestBTBUpdateThenLookupHits(t *testing.T) {
	b := NewBranchTargetBuffer(256)
	b.Update(0x01, 0x02, "conditional")
	result := b.Lookup(0x01)
	if result != 0x02 {
		t.Errorf("expected 0x02, got 0x%x", result)
	}
	if b.Hits != 1 {
		t.Error("expected 1 hit")
	}
}

func TestBTBAliasingCausesMiss(t *testing.T) {
	b := NewBranchTargetBuffer(256)
	// Two PCs that map to the same index: 0x01 and 0x01 + 256
	b.Update(0x01, 0x50, "conditional")
	b.Update(0x01+256, 0x60, "conditional")
	// The second evicts the first
	result := b.Lookup(0x01)
	if result != NoTarget {
		t.Error("expected miss due to aliasing eviction")
	}
}

func TestBTBGetEntry(t *testing.T) {
	b := NewBranchTargetBuffer(256)
	b.Update(0x01, 0x02, "call")
	entry := b.GetEntry(0x01)
	if entry == nil {
		t.Fatal("expected non-nil entry")
	}
	if !entry.Valid || entry.Tag != 0x01 || entry.Target != 0x02 || entry.BranchType != "call" {
		t.Errorf("unexpected entry: %+v", entry)
	}
}

func TestBTBGetEntryMiss(t *testing.T) {
	b := NewBranchTargetBuffer(256)
	entry := b.GetEntry(0x01)
	if entry != nil {
		t.Error("expected nil entry on miss")
	}
}

func TestBTBHitRateZeroLookups(t *testing.T) {
	b := NewBranchTargetBuffer(256)
	assertClose(t, b.HitRate(), 0.0, 0.001, "zero lookups hit rate")
}

func TestBTBHitRateAllHits(t *testing.T) {
	b := NewBranchTargetBuffer(256)
	b.Update(0x01, 0x02, "conditional")
	b.Lookup(0x01)
	b.Lookup(0x01)
	assertClose(t, b.HitRate(), 100.0, 0.001, "all hits")
}

func TestBTBHitRateMixed(t *testing.T) {
	b := NewBranchTargetBuffer(256)
	b.Update(0x01, 0x02, "conditional")
	b.Lookup(0x01) // hit
	b.Lookup(0x03) // miss
	assertClose(t, b.HitRate(), 50.0, 0.001, "mixed hit rate")
}

func TestBTBMultipleBranches(t *testing.T) {
	b := NewBranchTargetBuffer(256)
	b.Update(0x01, 0x10, "conditional")
	b.Update(0x02, 0x20, "unconditional")
	b.Update(0x03, 0x30, "call")
	if b.Lookup(0x01) != 0x10 {
		t.Error("wrong target for 0x01")
	}
	if b.Lookup(0x02) != 0x20 {
		t.Error("wrong target for 0x02")
	}
	if b.Lookup(0x03) != 0x30 {
		t.Error("wrong target for 0x03")
	}
}

func TestBTBUpdateOverwritesTarget(t *testing.T) {
	b := NewBranchTargetBuffer(256)
	b.Update(0x01, 0x10, "conditional")
	b.Update(0x01, 0x20, "conditional")
	if b.Lookup(0x01) != 0x20 {
		t.Error("expected updated target")
	}
}

func TestBTBReset(t *testing.T) {
	b := NewBranchTargetBuffer(256)
	b.Update(0x01, 0x02, "conditional")
	b.Lookup(0x01)
	b.Reset()
	if b.Lookups != 0 || b.Hits != 0 || b.Misses != 0 {
		t.Error("expected zero stats after reset")
	}
	if b.Lookup(0x01) != NoTarget {
		t.Error("expected miss after reset")
	}
}

func TestBTBDefaultBranchType(t *testing.T) {
	b := NewBranchTargetBuffer(256)
	b.Update(0x01, 0x02, "conditional")
	entry := b.GetEntry(0x01)
	if entry.BranchType != "conditional" {
		t.Errorf("expected conditional, got %s", entry.BranchType)
	}
}

func TestBTBReturnBranchType(t *testing.T) {
	b := NewBranchTargetBuffer(256)
	b.Update(0x01, 0x02, "return")
	entry := b.GetEntry(0x01)
	if entry.BranchType != "return" {
		t.Errorf("expected return, got %s", entry.BranchType)
	}
}

// ─── Interface Compliance Tests ────────────────────────────────────────────────
//
// These compile-time checks verify that all predictor types satisfy the
// BranchPredictor interface.

var _ BranchPredictor = (*AlwaysTakenPredictor)(nil)
var _ BranchPredictor = (*AlwaysNotTakenPredictor)(nil)
var _ BranchPredictor = (*BTFNTPredictor)(nil)
var _ BranchPredictor = (*OneBitPredictor)(nil)
var _ BranchPredictor = (*TwoBitPredictor)(nil)
