package neuralframework

import (
	"bytes"
	"encoding/json"
	"math"
	"reflect"
	"testing"
)

func single(t *testing.T, w, b float64, a Activation) *Network {
	t.Helper()
	n, e := New([]Layer{{a, Grid{{w}}, []float64{b}}})
	return must(t, n, e)
}
func TestFiniteDifferences(t *testing.T) {
	for _, a := range []Activation{Linear, Sigmoid, Tanh, ReLU} {
		t.Run(string(a), func(t *testing.T) {
			n, e := Seeded([]int{2, 3, 2, 2}, []Activation{a, a, Linear}, 7)
			n = must(t, n, e)
			// Positive offsets keep ReLU away from its nondifferentiable zero.
			for i := range n.layers {
				for j := range n.layers[i].Biases {
					n.layers[i].Biases[j] = 0.7
				}
			}
			x, y := Grid{{0.2, -0.1}, {0.4, 0.3}}, Grid{{0.2, 0.7}, {0.5, -0.2}}
			g, e := n.Gradients(x, y)
			if e != nil {
				t.Fatal(e)
			}
			loss := func(cp Checkpoint) float64 {
				p, e := FromCheckpoint(cp)
				p = must(t, p, e)
				v, e := p.Gradients(x, y)
				if e != nil {
					t.Fatal(e)
				}
				return v.Loss
			}
			const h = 1e-6
			for l := range n.layers {
				for r := range n.layers[l].Weights {
					for c := range n.layers[l].Weights[r] {
						plus, minus := n.ToCheckpoint(), n.ToCheckpoint()
						plus.Layers[l].Weights[r][c] += h
						minus.Layers[l].Weights[r][c] -= h
						closeNumber(t, g.WeightGradients[l][r][c], (loss(plus)-loss(minus))/(2*h), 2e-7)
					}
				}
				for j := range n.layers[l].Biases {
					plus, minus := n.ToCheckpoint(), n.ToCheckpoint()
					plus.Layers[l].Biases[j] += h
					minus.Layers[l].Biases[j] -= h
					closeNumber(t, g.BiasGradients[l][j], (loss(plus)-loss(minus))/(2*h), 2e-7)
				}
			}
		})
	}
	closeNumber(t, activate(-1000, Sigmoid), 0, 1e-12)
	closeNumber(t, activate(1000, Sigmoid), 1, 1e-12)
	if derivative(0, 0, ReLU) != 0 {
		t.Fatal("relu zero")
	}
}
func TestXORAndOwnership(t *testing.T) {
	var f struct {
		XOR struct {
			Inputs, Targets Grid
			Widths          []int
			Activations     []Activation
			Seed            uint32
			Epochs          int
			Rate            float64 `json:"learning_rate"`
		}
	}
	fixture(t, "dense.json", &f)
	x := f.XOR
	n, e := Seeded(x.Widths, x.Activations, x.Seed)
	n = must(t, n, e)
	before := n.ToCheckpoint()
	zero, e := n.Fit(x.Inputs, x.Targets, 0, x.Rate)
	if e != nil || len(zero) != 1 || !reflect.DeepEqual(before, n.ToCheckpoint()) {
		t.Fatal("zero epochs")
	}
	history, e := n.Fit(x.Inputs, x.Targets, x.Epochs, x.Rate)
	if e != nil || len(history) != 3001 || history[3000] >= 0.01 || n.Step() != 3000 {
		t.Fatalf("xor: %v", e)
	}
	data, e := n.ToJSON()
	if e != nil {
		t.Fatal(e)
	}
	resumed, e := FromJSON(data)
	resumed = must(t, resumed, e)
	for i := 0; i < 2; i++ {
		if _, e = n.TrainStep(x.Inputs, x.Targets, x.Rate); e != nil {
			t.Fatal(e)
		}
		if _, e = resumed.TrainStep(x.Inputs, x.Targets, x.Rate); e != nil {
			t.Fatal(e)
		}
		if !reflect.DeepEqual(n.ToCheckpoint(), resumed.ToCheckpoint()) {
			t.Fatal("resume")
		}
	}
	checkpoint := n.ToCheckpoint()
	restored, e := FromCheckpoint(checkpoint)
	restored = must(t, restored, e)
	checkpoint.Layers[0].Weights[0][0] = 999
	checkpoint.Layers[0].Biases[0] = 999
	original := n.ToCheckpoint()
	trace, e := n.Gradients(x.Inputs, x.Targets)
	if e != nil {
		t.Fatal(e)
	}
	trace.Predictions[0][0] = 999
	trace.WeightGradients[0][0][0] = 999
	trace.BiasGradients[0][0] = 999
	if !reflect.DeepEqual(original, n.ToCheckpoint()) || !reflect.DeepEqual(original, restored.ToCheckpoint()) {
		t.Fatal("alias")
	}
	layers := []Layer{{Linear, Grid{{1}}, []float64{0}}}
	owned, e := New(layers)
	owned = must(t, owned, e)
	layers[0].Weights[0][0] = 999
	if owned.ToCheckpoint().Layers[0].Weights[0][0] != 1 {
		t.Fatal("constructor alias")
	}
}
func TestInvalidConstruction(t *testing.T) {
	for _, ls := range [][]Layer{nil, {{Linear, nil, []float64{0}}}, {{Linear, Grid{{}}, []float64{0}}}, {{Linear, Grid{{1}, {1, 2}}, []float64{0}}}, {{"bad", Grid{{1}}, []float64{0}}}, {{Linear, Grid{{1}}, nil}}, {{Linear, Grid{{math.NaN()}}, []float64{0}}}, {{Linear, Grid{{1}}, []float64{math.Inf(1)}}}, {{Linear, Grid{{1}}, []float64{0}}, {Linear, Grid{{1}, {2}}, []float64{0}}}} {
		if _, e := New(ls); e == nil {
			t.Fatal("invalid layer")
		}
	}
	for _, c := range []struct {
		w []int
		a []Activation
	}{{nil, nil}, {[]int{1}, nil}, {[]int{1, 1}, nil}, {[]int{0, 1}, []Activation{Linear}}, {[]int{1, -1}, []Activation{Linear}}, {[]int{1, 1}, []Activation{"bad"}}, {[]int{math.MaxInt, 2}, []Activation{Linear}}} {
		if _, e := Seeded(c.w, c.a, 0); e == nil {
			t.Fatal("seed shapes")
		}
	}
	for _, edit := range []func(*Checkpoint){func(c *Checkpoint) { c.Format = "bad" }, func(c *Checkpoint) { c.Version = 2 }, func(c *Checkpoint) { c.Step = MaxStep + 1 }, func(c *Checkpoint) { c.Layers = nil }} {
		cp := single(t, 1, 0, Linear).ToCheckpoint()
		edit(&cp)
		if _, e := FromCheckpoint(cp); e == nil {
			t.Fatal("metadata")
		}
	}
	if capacity(math.MaxInt, 2) || capacity(1, 0) || capacity(-1, 1) {
		t.Fatal("capacity")
	}
}
func TestInvalidBatchesAndAtomicUpdates(t *testing.T) {
	n := single(t, 1, 0, Linear)
	before := n.ToCheckpoint()
	for _, x := range []Grid{nil, {{}}, {{1, 2}}, {{1}, {1, 2}}, {{math.NaN()}}, {{math.Inf(-1)}}} {
		if _, e := n.Predict(x); e == nil {
			t.Fatal("bad input")
		}
	}
	for _, y := range []Grid{nil, {{}}, {{1, 2}}, {{1}, {2}}, {{math.NaN()}}} {
		if _, e := n.Gradients(Grid{{1}}, y); e == nil {
			t.Fatal("bad target")
		}
	}
	for _, rate := range []float64{-1, 0, math.NaN(), math.Inf(1), math.MaxFloat64} {
		if _, e := n.TrainStep(Grid{{2}}, Grid{{0}}, rate); e == nil {
			t.Fatal("bad rate/overflow")
		}
	}
	for _, epochs := range []int{-1, math.MaxInt} {
		if _, e := n.Fit(Grid{{1}}, Grid{{0}}, epochs, 1); e == nil {
			t.Fatal("epochs")
		}
	}
	if _, e := n.Fit(Grid{{1}}, Grid{{0}}, 0, 0); e == nil {
		t.Fatal("fit rate")
	}
	if _, e := n.Fit(nil, nil, 1, 1); e == nil {
		t.Fatal("fit input")
	}
	if _, e := n.Fit(Grid{{1}}, Grid{{0}}, 1, math.MaxFloat64); e == nil {
		t.Fatal("fit atomic failure")
	}
	if !reflect.DeepEqual(n.ToCheckpoint(), before) {
		t.Fatal("failure mutation")
	}
	n.step = MaxStep
	if _, e := n.TrainStep(Grid{{1}}, Grid{{0}}, 1); e == nil || n.Step() != MaxStep {
		t.Fatal("step overflow")
	}
	if _, e := n.TrainStep(nil, nil, 1); e == nil {
		t.Fatal("nil batch")
	}
	var absent *Network
	if _, e := absent.Predict(Grid{{1}}); e == nil {
		t.Fatal("nil model")
	}
	if _, e := absent.TrainStep(nil, nil, 1); e == nil {
		t.Fatal("nil update")
	}
	if _, e := new(Network).Predict(Grid{{1}}); e == nil {
		t.Fatal("empty model")
	}
	// Finite candidate weights can still yield an overflowing batch loss.
	n = single(t, 0, 0, Linear)
	before = n.ToCheckpoint()
	if _, e := n.TrainStep(Grid{{1e100}}, Grid{{1e-100}}, 1e100); e == nil || !reflect.DeepEqual(before, n.ToCheckpoint()) {
		t.Fatal("candidate loss atomicity")
	}
	for _, pair := range []struct{ x, y Grid }{{Grid{{math.MaxFloat64}}, Grid{{-math.MaxFloat64}}}, {Grid{{math.MaxFloat64}}, Grid{{0}}}} {
		if _, e := single(t, 1, 0, Linear).Gradients(pair.x, pair.y); e == nil {
			t.Fatal("loss overflow")
		}
	}
	if _, e := single(t, math.MaxFloat64, math.MaxFloat64, Linear).Predict(Grid{{1}}); e == nil {
		t.Fatal("bias overflow")
	}
	if _, e := single(t, math.MaxFloat64, 0, Linear).Predict(Grid{{2}}); e == nil {
		t.Fatal("dot overflow")
	}
	if _, e := dot(Grid{{1}}, Grid{{1}, {2}}); e == nil {
		t.Fatal("dot shape")
	}
	if _, e := dot(nil, nil); e == nil {
		t.Fatal("dot empty")
	}
}
func TestJSONBoundary(t *testing.T) {
	for _, data := range [][]byte{nil, []byte("{"), []byte("{} {}"), []byte(`{"format":"coding-adventures-dense","version":1,"step":0,"layers":[{"activation":"linear","weights":[[null]],"biases":[0]}]}`), []byte(`{"format":"coding-adventures-dense","version":1,"step":0,"layers":[{"activation":"linear","weights":[[0]],"biases":[null]}]}`), bytes.Repeat([]byte(" "), MaxCheckpointBytes+1)} {
		if _, e := FromJSON(data); e == nil {
			t.Fatal("json accepted")
		}
	}
	if numericTokens([]json.RawMessage{[]byte("null")}) {
		t.Fatal("null number")
	}
}
