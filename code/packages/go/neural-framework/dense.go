// Package neuralframework implements native dense training above the matrix
// foundation. A layer computes A*W+b; backward walks the same layers in reverse.
// No external ML engine or other language owns these parameter updates.
// Networks are mutable and are not safe for concurrent mutation; snapshots own
// their arrays, so callers can inspect a trace without changing the model.
package neuralframework

import (
	"bytes"
	"encoding/json"
	"errors"
	"io"
	"math"

	"github.com/adhithyan15/coding-adventures/code/packages/go/matrix"
)

// MaxStep is the largest optimizer counter exactly interoperable with JS.
const MaxStep uint64 = 1<<53 - 1
const MaxCheckpointBytes = 4 * 1024 * 1024
const checkpointFormat = "coding-adventures-dense"

var ErrInvalid = errors.New("invalid dense model, batch, count, or nonfinite arithmetic")

type Activation string

const (
	Linear  Activation = "linear"
	Sigmoid Activation = "sigmoid"
	Tanh    Activation = "tanh"
	ReLU    Activation = "relu"
)

type Grid = [][]float64
type Layer struct {
	Activation Activation `json:"activation"`
	Weights    Grid       `json:"weights"`
	Biases     []float64  `json:"biases"`
}
type Checkpoint struct {
	Format  string  `json:"format"`
	Version int     `json:"version"`
	Step    uint64  `json:"step"`
	Layers  []Layer `json:"layers"`
}
type GradientTrace struct {
	Loss            float64
	Predictions     Grid
	WeightGradients []Grid
	BiasGradients   [][]float64
}
type Network struct {
	layers []Layer
	step   uint64
}

func finite(v float64) bool             { return !math.IsNaN(v) && !math.IsInf(v, 0) }
func validActivation(a Activation) bool { return a == Linear || a == Sigmoid || a == Tanh || a == ReLU }

// Check multiplication before allocation, never after a narrowing conversion.
func capacity(rows, cols int) bool {
	return rows > 0 && cols > 0 && rows <= math.MaxInt/24 && rows <= math.MaxInt/8/cols
}
func validGrid(g Grid) bool {
	if len(g) == 0 || !capacity(len(g), len(g[0])) {
		return false
	}
	for _, row := range g {
		if len(row) != len(g[0]) {
			return false
		}
		for _, v := range row {
			if !finite(v) {
				return false
			}
		}
	}
	return true
}
func copyGrid(g Grid) Grid {
	out := make(Grid, len(g))
	for i, row := range g {
		out[i] = append([]float64(nil), row...)
	}
	return out
}
func copyLayers(layers []Layer) []Layer {
	out := make([]Layer, len(layers))
	for i, l := range layers {
		out[i] = Layer{l.Activation, copyGrid(l.Weights), append([]float64(nil), l.Biases...)}
	}
	return out
}
func zeros(rows, cols int) Grid {
	g := make(Grid, rows)
	for i := range g {
		g[i] = make([]float64, cols)
	}
	return g
}

// New validates the entire topology before taking owned copies. Matrix's public
// shape fields are never accepted as evidence that caller data is rectangular.
func New(layers []Layer) (*Network, error) {
	if len(layers) == 0 {
		return nil, ErrInvalid
	}
	for i, l := range layers {
		if !validActivation(l.Activation) || !validGrid(l.Weights) || len(l.Biases) != len(l.Weights[0]) {
			return nil, ErrInvalid
		}
		for _, b := range l.Biases {
			if !finite(b) {
				return nil, ErrInvalid
			}
		}
		if i > 0 && len(l.Weights) != len(layers[i-1].Biases) {
			return nil, ErrInvalid
		}
	}
	return &Network{layers: copyLayers(layers)}, nil
}

// Seeded uses NN37's uint32 wrapping LCG, not a cryptographic random stream.
func Seeded(widths []int, kinds []Activation, seed uint32) (*Network, error) {
	if len(widths) < 2 || len(kinds) != len(widths)-1 {
		return nil, ErrInvalid
	}
	for i, a := range kinds {
		if !validActivation(a) || !capacity(widths[i], widths[i+1]) {
			return nil, ErrInvalid
		}
	}
	layers := make([]Layer, len(kinds))
	state := seed
	for i, a := range kinds {
		w := zeros(widths[i], widths[i+1])
		for r := range w {
			for c := range w[r] {
				state = 1664525*state + 1013904223
				w[r][c] = (2*float64(state)/4294967296 - 1) / math.Sqrt(float64(widths[i]))
			}
		}
		layers[i] = Layer{a, w, make([]float64, widths[i+1])}
	}
	return New(layers)
}
func activate(z float64, a Activation) float64 {
	switch a {
	case Linear:
		return z
	case ReLU:
		return math.Max(0, z)
	case Tanh:
		return math.Tanh(z)
	default:
		f := math.Exp(-math.Abs(z))
		if z >= 0 {
			return 1 / (1 + f)
		}
		return f / (1 + f)
	}
}
func derivative(z, a float64, kind Activation) float64 {
	switch kind {
	case Linear:
		return 1
	case ReLU:
		if z > 0 {
			return 1
		}
		return 0
	case Tanh:
		return 1 - a*a
	default:
		return a * (1 - a)
	}
}

// Matrix operations are used only on internally validated dimensions. Their
// output is still checked: finite inputs can overflow during a dot product.
func dot(a, b Grid) (Grid, error) {
	if !validGrid(a) || !validGrid(b) || len(a[0]) != len(b) || !capacity(len(a), len(b[0])) {
		return nil, ErrInvalid
	}
	m, e := matrix.New2D(a).Dot(matrix.New2D(b))
	if e != nil || m == nil || !validGrid(m.Data) {
		return nil, ErrInvalid
	}
	return m.Data, nil
}
func transpose(g Grid) Grid { return matrix.New2D(g).Transpose().Data }
func (n *Network) forward(inputs Grid) ([]Grid, []Grid, error) {
	if n == nil || len(n.layers) == 0 || !validGrid(inputs) || len(inputs[0]) != len(n.layers[0].Weights) {
		return nil, nil, ErrInvalid
	}
	current := copyGrid(inputs)
	saved := []Grid{current}
	raws := make([]Grid, 0, len(n.layers))
	for _, l := range n.layers {
		raw, e := dot(current, l.Weights)
		if e != nil {
			return nil, nil, e
		}
		next := zeros(len(raw), len(raw[0]))
		for i := range raw {
			for j := range raw[i] {
				raw[i][j] += l.Biases[j]
				next[i][j] = activate(raw[i][j], l.Activation)
			}
		}
		if !validGrid(raw) || !validGrid(next) {
			return nil, nil, ErrInvalid
		}
		raws = append(raws, raw)
		saved = append(saved, next)
		current = next
	}
	return saved, raws, nil
}
func (n *Network) Predict(inputs Grid) (Grid, error) {
	saved, _, e := n.forward(inputs)
	if e != nil {
		return nil, e
	}
	return saved[len(saved)-1], nil
}

// Gradients returns the pre-update trace. MSE divides by samples*outputs;
// forgetting outputs would silently change the effective learning rate.
func (n *Network) Gradients(inputs, targets Grid) (GradientTrace, error) {
	saved, raw, e := n.forward(inputs)
	if e != nil {
		return GradientTrace{}, e
	}
	p := saved[len(saved)-1]
	if !validGrid(targets) || len(targets) != len(p) || len(targets[0]) != len(p[0]) {
		return GradientTrace{}, ErrInvalid
	}
	count := float64(len(p)) * float64(len(p[0]))
	up := zeros(len(p), len(p[0]))
	loss := 0.0
	for i := range p {
		for j := range p[i] {
			d := p[i][j] - targets[i][j]
			loss += d * d
			up[i][j] = 2 * d / count
		}
	}
	loss /= count
	if !finite(loss) || !validGrid(up) {
		return GradientTrace{}, ErrInvalid
	}
	trace := GradientTrace{loss, p, make([]Grid, len(n.layers)), make([][]float64, len(n.layers))}
	for index := len(n.layers) - 1; index >= 0; index-- {
		l := n.layers[index]
		delta := zeros(len(up), len(up[0]))
		for i := range up {
			for j := range up[i] {
				delta[i][j] = up[i][j] * derivative(raw[index][i][j], saved[index+1][i][j], l.Activation)
			}
		}
		if !validGrid(delta) {
			return GradientTrace{}, ErrInvalid
		}
		dw, e := dot(transpose(saved[index]), delta)
		if e != nil {
			return GradientTrace{}, e
		}
		db := make([]float64, len(l.Biases))
		for _, row := range delta {
			for j, v := range row {
				db[j] += v
			}
		}
		for _, b := range db {
			if !finite(b) {
				return GradientTrace{}, ErrInvalid
			}
		}
		trace.WeightGradients[index] = dw
		trace.BiasGradients[index] = db
		if index > 0 {
			up, e = dot(delta, transpose(l.Weights))
			if e != nil {
				return GradientTrace{}, e
			}
		}
	}
	return trace, nil
}

// TrainStep is transactional: even finite candidate parameters can produce an
// overflowing prediction/loss. Evaluate that candidate before adopting any bit.
func (n *Network) TrainStep(inputs, targets Grid, rate float64) (GradientTrace, error) {
	if n == nil || !finite(rate) || rate <= 0 || n.step >= MaxStep {
		return GradientTrace{}, ErrInvalid
	}
	trace, e := n.Gradients(inputs, targets)
	if e != nil {
		return GradientTrace{}, e
	}
	layers := copyLayers(n.layers)
	for i := range layers {
		for r := range layers[i].Weights {
			for c := range layers[i].Weights[r] {
				layers[i].Weights[r][c] -= rate * trace.WeightGradients[i][r][c]
			}
		}
		for j := range layers[i].Biases {
			layers[i].Biases[j] -= rate * trace.BiasGradients[i][j]
		}
	}
	candidate, e := New(layers)
	if e != nil {
		return GradientTrace{}, e
	}
	candidate.step = n.step + 1
	if _, e = candidate.Gradients(inputs, targets); e != nil {
		return GradientTrace{}, e
	}
	n.layers, n.step = candidate.layers, candidate.step
	return trace, nil
}
func (n *Network) Fit(inputs, targets Grid, epochs int, rate float64) ([]float64, error) {
	if epochs < 0 || epochs >= math.MaxInt/8 || uint64(epochs) > MaxStep || !finite(rate) || rate <= 0 {
		return nil, ErrInvalid
	}
	g, e := n.Gradients(inputs, targets)
	if e != nil {
		return nil, e
	}
	losses := []float64{g.Loss}
	for i := 0; i < epochs; i++ {
		if _, e = n.TrainStep(inputs, targets, rate); e != nil {
			return nil, e
		}
		g, e = n.Gradients(inputs, targets)
		if e != nil {
			return nil, e
		}
		losses = append(losses, g.Loss)
	}
	return losses, nil
}
func (n *Network) Step() uint64 { return n.step }
func (n *Network) ToCheckpoint() Checkpoint {
	return Checkpoint{checkpointFormat, 1, n.step, copyLayers(n.layers)}
}
func FromCheckpoint(c Checkpoint) (*Network, error) {
	if c.Format != checkpointFormat || c.Version != 1 || c.Step > MaxStep {
		return nil, ErrInvalid
	}
	n, e := New(c.Layers)
	if e != nil {
		return nil, e
	}
	n.step = c.Step
	return n, nil
}
func (n *Network) ToJSON() ([]byte, error) { return json.Marshal(n.ToCheckpoint()) }

// JSON's ordinary struct decoder supplies zero values for absent fields. First
// check exact object keys, then decode typed records so null, strings and bools
// cannot become valid numbers. No checkpoint field selects executable code.
func keys(raw json.RawMessage, names ...string) (map[string]json.RawMessage, error) {
	var m map[string]json.RawMessage
	if e := json.Unmarshal(raw, &m); e != nil || m == nil || len(m) != len(names) {
		return nil, ErrInvalid
	}
	for _, name := range names {
		v, ok := m[name]
		if !ok || bytes.Equal(bytes.TrimSpace(v), []byte("null")) {
			return nil, ErrInvalid
		}
	}
	return m, nil
}
func FromJSON(data []byte) (*Network, error) {
	if len(data) > MaxCheckpointBytes {
		return nil, ErrInvalid
	}
	m, e := keys(data, "format", "version", "step", "layers")
	if e != nil {
		return nil, e
	}
	var layers []json.RawMessage
	if e = json.Unmarshal(m["layers"], &layers); e != nil {
		return nil, ErrInvalid
	}
	for _, l := range layers {
		fields, e := keys(l, "activation", "weights", "biases")
		if e != nil {
			return nil, e
		}
		// encoding/json accepts null into a float64 as zero. Admit only actual
		// numeric JSON tokens before the typed decoder sees any parameter cell.
		var weights [][]json.RawMessage
		if json.Unmarshal(fields["weights"], &weights) != nil {
			return nil, ErrInvalid
		}
		for _, row := range weights {
			if !numericTokens(row) {
				return nil, ErrInvalid
			}
		}
		var biases []json.RawMessage
		if json.Unmarshal(fields["biases"], &biases) != nil || !numericTokens(biases) {
			return nil, ErrInvalid
		}
	}
	var c Checkpoint
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.DisallowUnknownFields()
	if e = decoder.Decode(&c); e != nil {
		return nil, ErrInvalid
	}
	if e = decoder.Decode(new(any)); e != io.EOF {
		return nil, ErrInvalid
	}
	return FromCheckpoint(c)
}

func numericTokens(values []json.RawMessage) bool {
	for _, raw := range values {
		var v float64
		if bytes.Equal(bytes.TrimSpace(raw), []byte("null")) || json.Unmarshal(raw, &v) != nil || !finite(v) {
			return false
		}
	}
	return true
}
