package branchpredictor

import (
	statemachine "github.com/adhithyan15/coding-adventures/code/packages/go/state-machine"
)

// NewOneBitDFA creates a DFA that models the one-bit branch predictor.
func NewOneBitDFA() *statemachine.DFA {
	result, _ := StartNew[*statemachine.DFA]("branch-predictor.NewOneBitDFA", nil,
		func(op *Operation[*statemachine.DFA], rf *ResultFactory[*statemachine.DFA]) *OperationResult[*statemachine.DFA] {
			dfa := statemachine.NewDFA(
				[]string{"NT", "T"},
				[]string{"taken", "not_taken"},
				map[[2]string]string{
					{"NT", "taken"}: "T", {"NT", "not_taken"}: "NT",
					{"T", "taken"}: "T", {"T", "not_taken"}: "NT",
				},
				"NT",
				[]string{"T"},
				nil,
			)
			return rf.Generate(true, false, dfa)
		}).GetResult()
	return result
}

// OneBitPredictor is a 1-bit dynamic predictor -- one flip-flop per branch.
type OneBitPredictor struct {
	tableSize int
	table     map[int]bool
	stats     PredictionStats
}

// NewOneBitPredictor creates a new 1-bit predictor.
func NewOneBitPredictor(tableSize int) *OneBitPredictor {
	result, _ := StartNew[*OneBitPredictor]("branch-predictor.NewOneBitPredictor", nil,
		func(op *Operation[*OneBitPredictor], rf *ResultFactory[*OneBitPredictor]) *OperationResult[*OneBitPredictor] {
			op.AddProperty("tableSize", tableSize)
			return rf.Generate(true, false, &OneBitPredictor{
				tableSize: tableSize,
				table:     make(map[int]bool),
			})
		}).GetResult()
	return result
}

// Predict returns a prediction based on the last outcome of this branch.
func (p *OneBitPredictor) Predict(pc int) Prediction {
	result, _ := StartNew[Prediction]("branch-predictor.OneBitPredictor.Predict", Prediction{},
		func(op *Operation[Prediction], rf *ResultFactory[Prediction]) *OperationResult[Prediction] {
			op.AddProperty("pc", pc)
			index := pc % p.tableSize
			taken, exists := p.table[index]
			if !exists {
				taken = false
			}
			return rf.Generate(true, false, Prediction{Taken: taken, Confidence: 0.5, Target: NoTarget})
		}).GetResult()
	return result
}

// Update records the actual outcome and sets the bit to match.
func (p *OneBitPredictor) Update(pc int, taken bool, _ int) {
	_, _ = StartNew[struct{}]("branch-predictor.OneBitPredictor.Update", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("pc", pc)
			op.AddProperty("taken", taken)
			index := pc % p.tableSize
			predicted, exists := p.table[index]
			if !exists {
				predicted = false
			}
			p.stats.Record(predicted == taken)
			p.table[index] = taken
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Stats returns prediction accuracy statistics.
func (p *OneBitPredictor) Stats() *PredictionStats {
	result, _ := StartNew[*PredictionStats]("branch-predictor.OneBitPredictor.Stats", nil,
		func(op *Operation[*PredictionStats], rf *ResultFactory[*PredictionStats]) *OperationResult[*PredictionStats] {
			return rf.Generate(true, false, &p.stats)
		}).GetResult()
	return result
}

// Reset clears the prediction table and statistics.
func (p *OneBitPredictor) Reset() {
	_, _ = StartNew[struct{}]("branch-predictor.OneBitPredictor.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			p.table = make(map[int]bool)
			p.stats.Reset()
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}
