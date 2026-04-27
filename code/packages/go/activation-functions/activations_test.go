package activation

import (
	"math"
	"testing"
)

func assertClose(t *testing.T, expected, actual float64) {
	t.Helper()
	if math.Abs(expected-actual) > 1e-12 {
		t.Fatalf("expected %.17g, got %.17g", expected, actual)
	}
}

func TestLinear(t *testing.T) {
	assertClose(t, -3.0, Linear(-3.0))
	assertClose(t, 0.0, Linear(0.0))
	assertClose(t, 5.0, Linear(5.0))
	assertClose(t, 1.0, LinearDerivative(-3.0))
	assertClose(t, 1.0, LinearDerivative(0.0))
	assertClose(t, 1.0, LinearDerivative(5.0))
}

func TestSigmoid(t *testing.T) {
	assertClose(t, 0.5, Sigmoid(0.0))
	assertClose(t, 0.7310585786300049, Sigmoid(1.0))
	assertClose(t, 0.2689414213699951, Sigmoid(-1.0))
	assertClose(t, 0.9999546021312976, Sigmoid(10.0))
	assertClose(t, 0.0, Sigmoid(-710.0))
	assertClose(t, 1.0, Sigmoid(710.0))
	assertClose(t, 0.25, SigmoidDerivative(0.0))
	assertClose(t, 0.19661193324148185, SigmoidDerivative(1.0))
}

func TestRelu(t *testing.T) {
	assertClose(t, 5.0, Relu(5.0))
	assertClose(t, 0.0, Relu(-3.0))
	assertClose(t, 0.0, Relu(0.0))
	assertClose(t, 1.0, ReluDerivative(5.0))
	assertClose(t, 0.0, ReluDerivative(-3.0))
	assertClose(t, 0.0, ReluDerivative(0.0))
}

func TestLeakyRelu(t *testing.T) {
	assertClose(t, 5.0, LeakyRelu(5.0))
	assertClose(t, -0.03, LeakyRelu(-3.0))
	assertClose(t, 0.0, LeakyRelu(0.0))
	assertClose(t, 1.0, LeakyReluDerivative(5.0))
	assertClose(t, 0.01, LeakyReluDerivative(-3.0))
	assertClose(t, 0.01, LeakyReluDerivative(0.0))
}

func TestTanh(t *testing.T) {
	assertClose(t, 0.0, Tanh(0.0))
	assertClose(t, 0.7615941559557649, Tanh(1.0))
	assertClose(t, -0.7615941559557649, Tanh(-1.0))
	assertClose(t, 1.0, TanhDerivative(0.0))
	assertClose(t, 0.41997434161402614, TanhDerivative(1.0))
}

func TestSoftplus(t *testing.T) {
	assertClose(t, 0.6931471805599453, Softplus(0.0))
	assertClose(t, 1.3132616875182228, Softplus(1.0))
	assertClose(t, 0.31326168751822286, Softplus(-1.0))
	if Softplus(1000.0) <= 999.0 {
		t.Fatalf("softplus should remain stable for large positive inputs")
	}
	assertClose(t, 0.5, SoftplusDerivative(0.0))
	assertClose(t, Sigmoid(1.0), SoftplusDerivative(1.0))
	assertClose(t, Sigmoid(-1.0), SoftplusDerivative(-1.0))
}
