package branchpredictor

import (
	statemachine "github.com/adhithyan15/coding-adventures/code/packages/go/state-machine"
)

// TwoBitState represents the 4 states of a 2-bit saturating counter.
type TwoBitState int

const (
	StronglyNotTaken TwoBitState = 0
	WeaklyNotTaken   TwoBitState = 1
	WeaklyTaken      TwoBitState = 2
	StronglyTaken    TwoBitState = 3
)

var twoBitDFAStateNames = map[TwoBitState]string{
	StronglyNotTaken: "SNT",
	WeaklyNotTaken:   "WNT",
	WeaklyTaken:      "WT",
	StronglyTaken:    "ST",
}

var twoBitDFAStateFromName = map[string]TwoBitState{
	"SNT": StronglyNotTaken,
	"WNT": WeaklyNotTaken,
	"WT":  WeaklyTaken,
	"ST":  StronglyTaken,
}

// NewTwoBitDFA creates a DFA that models the two-bit saturating counter.
func NewTwoBitDFA() *statemachine.DFA {
	result, _ := StartNew[*statemachine.DFA]("branch-predictor.NewTwoBitDFA", nil,
		func(op *Operation[*statemachine.DFA], rf *ResultFactory[*statemachine.DFA]) *OperationResult[*statemachine.DFA] {
			dfa := statemachine.NewDFA(
				[]string{"SNT", "WNT", "WT", "ST"},
				[]string{"taken", "not_taken"},
				map[[2]string]string{
					{"SNT", "taken"}: "WNT", {"SNT", "not_taken"}: "SNT",
					{"WNT", "taken"}: "WT", {"WNT", "not_taken"}: "SNT",
					{"WT", "taken"}: "ST", {"WT", "not_taken"}: "WNT",
					{"ST", "taken"}: "ST", {"ST", "not_taken"}: "WT",
				},
				"WNT",
				[]string{"WT", "ST"},
				nil,
			)
			return rf.Generate(true, false, dfa)
		}).GetResult()
	return result
}

// TwoBitStateName returns the DFA state name for a TwoBitState value.
func TwoBitStateName(s TwoBitState) string {
	result, _ := StartNew[string]("branch-predictor.TwoBitStateName", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("s", s)
			return rf.Generate(true, false, twoBitDFAStateNames[s])
		}).GetResult()
	return result
}

// TwoBitStateFromName returns the TwoBitState value for a DFA state name.
func TwoBitStateFromName(name string) TwoBitState {
	result, _ := StartNew[TwoBitState]("branch-predictor.TwoBitStateFromName", StronglyNotTaken,
		func(op *Operation[TwoBitState], rf *ResultFactory[TwoBitState]) *OperationResult[TwoBitState] {
			op.AddProperty("name", name)
			s, ok := twoBitDFAStateFromName[name]
			if !ok {
				return rf.Generate(true, false, StronglyNotTaken)
			}
			return rf.Generate(true, false, s)
		}).GetResult()
	return result
}

// TakenOutcome returns the next state after a "taken" branch outcome.
func (s TwoBitState) TakenOutcome() TwoBitState {
	result, _ := StartNew[TwoBitState]("branch-predictor.TwoBitState.TakenOutcome", StronglyNotTaken,
		func(op *Operation[TwoBitState], rf *ResultFactory[TwoBitState]) *OperationResult[TwoBitState] {
			if s >= StronglyTaken {
				return rf.Generate(true, false, StronglyTaken)
			}
			return rf.Generate(true, false, s+1)
		}).GetResult()
	return result
}

// NotTakenOutcome returns the next state after a "not taken" branch outcome.
func (s TwoBitState) NotTakenOutcome() TwoBitState {
	result, _ := StartNew[TwoBitState]("branch-predictor.TwoBitState.NotTakenOutcome", StronglyNotTaken,
		func(op *Operation[TwoBitState], rf *ResultFactory[TwoBitState]) *OperationResult[TwoBitState] {
			if s <= StronglyNotTaken {
				return rf.Generate(true, false, StronglyNotTaken)
			}
			return rf.Generate(true, false, s-1)
		}).GetResult()
	return result
}

// PredictsTaken returns whether this state predicts "taken".
func (s TwoBitState) PredictsTaken() bool {
	result, _ := StartNew[bool]("branch-predictor.TwoBitState.PredictsTaken", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, s >= WeaklyTaken)
		}).GetResult()
	return result
}

// TwoBitPredictor is a 2-bit saturating counter predictor.
type TwoBitPredictor struct {
	tableSize    int
	initialState TwoBitState
	table        map[int]TwoBitState
	stats        PredictionStats
}

// NewTwoBitPredictor creates a new 2-bit predictor.
func NewTwoBitPredictor(tableSize int, initialState TwoBitState) *TwoBitPredictor {
	result, _ := StartNew[*TwoBitPredictor]("branch-predictor.NewTwoBitPredictor", nil,
		func(op *Operation[*TwoBitPredictor], rf *ResultFactory[*TwoBitPredictor]) *OperationResult[*TwoBitPredictor] {
			op.AddProperty("tableSize", tableSize)
			return rf.Generate(true, false, &TwoBitPredictor{
				tableSize:    tableSize,
				initialState: initialState,
				table:        make(map[int]TwoBitState),
			})
		}).GetResult()
	return result
}

func (p *TwoBitPredictor) getState(index int) TwoBitState {
	state, exists := p.table[index]
	if !exists {
		return p.initialState
	}
	return state
}

// Predict returns a prediction based on the 2-bit counter for this branch.
func (p *TwoBitPredictor) Predict(pc int) Prediction {
	result, _ := StartNew[Prediction]("branch-predictor.TwoBitPredictor.Predict", Prediction{},
		func(op *Operation[Prediction], rf *ResultFactory[Prediction]) *OperationResult[Prediction] {
			op.AddProperty("pc", pc)
			index := pc % p.tableSize
			state := p.getState(index)
			confidence := 0.5
			if state == StronglyTaken || state == StronglyNotTaken {
				confidence = 1.0
			}
			return rf.Generate(true, false, Prediction{Taken: state.PredictsTaken(), Confidence: confidence, Target: NoTarget})
		}).GetResult()
	return result
}

// Update transitions the 2-bit counter based on the actual outcome.
func (p *TwoBitPredictor) Update(pc int, taken bool, _ int) {
	_, _ = StartNew[struct{}]("branch-predictor.TwoBitPredictor.Update", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("pc", pc)
			op.AddProperty("taken", taken)
			index := pc % p.tableSize
			state := p.getState(index)
			p.stats.Record(state.PredictsTaken() == taken)
			if taken {
				p.table[index] = state.TakenOutcome()
			} else {
				p.table[index] = state.NotTakenOutcome()
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Stats returns prediction accuracy statistics.
func (p *TwoBitPredictor) Stats() *PredictionStats {
	result, _ := StartNew[*PredictionStats]("branch-predictor.TwoBitPredictor.Stats", nil,
		func(op *Operation[*PredictionStats], rf *ResultFactory[*PredictionStats]) *OperationResult[*PredictionStats] {
			return rf.Generate(true, false, &p.stats)
		}).GetResult()
	return result
}

// Reset clears the prediction table and statistics.
func (p *TwoBitPredictor) Reset() {
	_, _ = StartNew[struct{}]("branch-predictor.TwoBitPredictor.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			p.table = make(map[int]TwoBitState)
			p.stats.Reset()
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// GetState returns the current TwoBitState for a branch address (for testing).
func (p *TwoBitPredictor) GetState(pc int) TwoBitState {
	result, _ := StartNew[TwoBitState]("branch-predictor.TwoBitPredictor.GetState", StronglyNotTaken,
		func(op *Operation[TwoBitState], rf *ResultFactory[TwoBitState]) *OperationResult[TwoBitState] {
			op.AddProperty("pc", pc)
			return rf.Generate(true, false, p.getState(pc%p.tableSize))
		}).GetResult()
	return result
}
