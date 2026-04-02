package branchpredictor

// BTBEntry represents a single entry in the Branch Target Buffer.
type BTBEntry struct {
	Valid      bool
	Tag        int
	Target     int
	BranchType string
}

// BranchTargetBuffer caches branch target addresses alongside a direction
// predictor to provide zero-bubble fetch redirection.
type BranchTargetBuffer struct {
	size    int
	entries []BTBEntry

	Lookups int
	Hits    int
	Misses  int
}

// NewBranchTargetBuffer creates a new BTB with the given number of entries.
func NewBranchTargetBuffer(size int) *BranchTargetBuffer {
	result, _ := StartNew[*BranchTargetBuffer]("branch-predictor.NewBranchTargetBuffer", nil,
		func(op *Operation[*BranchTargetBuffer], rf *ResultFactory[*BranchTargetBuffer]) *OperationResult[*BranchTargetBuffer] {
			op.AddProperty("size", size)
			entries := make([]BTBEntry, size)
			return rf.Generate(true, false, &BranchTargetBuffer{size: size, entries: entries})
		}).GetResult()
	return result
}

// Lookup returns the cached target for a branch at pc, or NoTarget on a miss.
func (b *BranchTargetBuffer) Lookup(pc int) int {
	result, _ := StartNew[int]("branch-predictor.BranchTargetBuffer.Lookup", NoTarget,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("pc", pc)
			b.Lookups++
			index := pc % b.size
			entry := b.entries[index]
			if entry.Valid && entry.Tag == pc {
				b.Hits++
				return rf.Generate(true, false, entry.Target)
			}
			b.Misses++
			return rf.Generate(true, false, NoTarget)
		}).GetResult()
	return result
}

// Update records a branch target after execution.
func (b *BranchTargetBuffer) Update(pc int, target int, branchType string) {
	_, _ = StartNew[struct{}]("branch-predictor.BranchTargetBuffer.Update", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("pc", pc)
			op.AddProperty("target", target)
			op.AddProperty("branchType", branchType)
			index := pc % b.size
			b.entries[index] = BTBEntry{
				Valid:      true,
				Tag:        pc,
				Target:     target,
				BranchType: branchType,
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// GetEntry returns the BTB entry for a given PC if it exists, or nil otherwise.
func (b *BranchTargetBuffer) GetEntry(pc int) *BTBEntry {
	result, _ := StartNew[*BTBEntry]("branch-predictor.BranchTargetBuffer.GetEntry", nil,
		func(op *Operation[*BTBEntry], rf *ResultFactory[*BTBEntry]) *OperationResult[*BTBEntry] {
			op.AddProperty("pc", pc)
			index := pc % b.size
			entry := b.entries[index]
			if entry.Valid && entry.Tag == pc {
				return rf.Generate(true, false, &entry)
			}
			return rf.Generate(true, false, nil)
		}).GetResult()
	return result
}

// HitRate returns the BTB hit rate as a percentage (0.0 to 100.0).
func (b *BranchTargetBuffer) HitRate() float64 {
	result, _ := StartNew[float64]("branch-predictor.BranchTargetBuffer.HitRate", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if b.Lookups == 0 {
				return rf.Generate(true, false, 0.0)
			}
			return rf.Generate(true, false, (float64(b.Hits)/float64(b.Lookups))*100.0)
		}).GetResult()
	return result
}

// Reset clears all BTB state -- entries and statistics.
func (b *BranchTargetBuffer) Reset() {
	_, _ = StartNew[struct{}]("branch-predictor.BranchTargetBuffer.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			b.entries = make([]BTBEntry, b.size)
			b.Lookups = 0
			b.Hits = 0
			b.Misses = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}
