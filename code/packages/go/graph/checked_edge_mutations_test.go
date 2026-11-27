package graph

import (
	"errors"
	"math"
	"reflect"
	"testing"
)

func TestAddEdgeCheckedRejectsInvalidWeightWithoutMutation(t *testing.T) {
	invalidWeights := []float64{-1, math.NaN(), math.Inf(1), math.Inf(-1)}
	for _, repr := range representations() {
		for _, weight := range invalidWeights {
			g := New(repr)
			if err := g.AddEdgeChecked("left", "right", weight); !errors.Is(err, ErrInvalidEdgeWeight) {
				t.Fatalf("AddEdgeChecked(%v) error = %v, want ErrInvalidEdgeWeight", weight, err)
			}
			if g.Size() != 0 || len(g.Edges()) != 0 || g.HasNode("left") || g.HasNode("right") {
				t.Fatalf("AddEdgeChecked(%v) mutated graph for repr=%s: nodes=%v edges=%v", weight, repr, g.Nodes(), g.Edges())
			}
		}
	}
}

func TestAddEdgeCheckedUsesPositionalWeightAndLegacyDefault(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			g := New(repr)
			if err := g.AddEdgeChecked("zero-left", "zero-right", 0); err != nil {
				t.Fatalf("AddEdgeChecked zero default: %v", err)
			}
			if weight, _ := g.EdgeWeight("zero-left", "zero-right"); weight != 1 {
				t.Fatalf("zero positional weight stored as %v, want default 1", weight)
			}

			if err := g.AddEdgeChecked("weighted-left", "weighted-right", 2.5, PropertyBag{"weight": math.NaN(), "kind": "route"}); err != nil {
				t.Fatalf("AddEdgeChecked should validate positional weight only: %v", err)
			}
			properties, err := g.EdgeProperties("weighted-left", "weighted-right")
			if err != nil {
				t.Fatalf("EdgeProperties: %v", err)
			}
			if properties["weight"] != 2.5 || properties["kind"] != "route" {
				t.Fatalf("canonical properties = %#v, want positional weight 2.5 and route kind", properties)
			}
		})
	}
}

func TestSetEdgePropertyCheckedValidatesAtomicallyAndPreservesPrecedence(t *testing.T) {
	invalidWeights := []float64{-2, math.NaN(), math.Inf(1), math.Inf(-1)}
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			g := New(repr)
			g.AddEdge("left", "right", 4, PropertyBag{"kind": "road"})
			before, err := g.EdgeProperties("left", "right")
			if err != nil {
				t.Fatal(err)
			}

			for _, weight := range invalidWeights {
				if err := g.SetEdgePropertyChecked("left", "right", "weight", weight); !errors.Is(err, ErrInvalidEdgeWeight) {
					t.Fatalf("SetEdgePropertyChecked(%v) error = %v, want ErrInvalidEdgeWeight", weight, err)
				}
				after, err := g.EdgeProperties("left", "right")
				if err != nil || !reflect.DeepEqual(after, before) {
					t.Fatalf("invalid update changed properties: got %#v, %v; want %#v", after, err, before)
				}
			}

			if err := g.SetEdgePropertyChecked("left", "right", "weight", "heavy"); err == nil {
				t.Fatal("nonnumeric canonical weight should fail")
			}
			after, err := g.EdgeProperties("left", "right")
			if err != nil || !reflect.DeepEqual(after, before) {
				t.Fatalf("nonnumeric update changed properties: got %#v, %v; want %#v", after, err, before)
			}

			var missing *EdgeNotFoundError
			err = g.SetEdgePropertyChecked("missing", "edge", "weight", math.NaN())
			if !errors.As(err, &missing) || errors.Is(err, ErrInvalidEdgeWeight) {
				t.Fatalf("missing edge vs invalid weight error = %v, want EdgeNotFoundError first", err)
			}

			if err := g.SetEdgePropertyChecked("left", "right", "weight", 0); err != nil {
				t.Fatalf("zero weight default update: %v", err)
			}
			if weight, _ := g.EdgeWeight("left", "right"); weight != 1 {
				t.Fatalf("zero property update stored %v, want default 1", weight)
			}
			if err := g.SetEdgePropertyChecked("left", "right", "weight", int32(7)); err != nil {
				t.Fatalf("valid numeric update: %v", err)
			}
			if weight, _ := g.EdgeWeight("left", "right"); weight != 7 {
				t.Fatalf("valid property update stored %v, want 7", weight)
			}
			if err := g.SetEdgePropertyChecked("left", "right", "note", math.NaN()); err != nil {
				t.Fatalf("non-weight properties keep existing semantics: %v", err)
			}
		})
	}
}

func TestSetEdgePropertyCheckedAcceptsSupportedNumericTypes(t *testing.T) {
	values := []any{
		int(3), int8(3), int16(3), int32(3), int64(3),
		uint(3), uint8(3), uint16(3), uint32(3), uint64(3),
		float32(3), float64(3),
	}
	for _, repr := range representations() {
		for _, value := range values {
			g := New(repr)
			g.AddEdge("left", "right", 1)
			if err := g.SetEdgePropertyChecked("left", "right", "weight", value); err != nil {
				t.Fatalf("SetEdgePropertyChecked(%T(%v)) for repr=%s: %v", value, value, repr, err)
			}
			if weight, _ := g.EdgeWeight("left", "right"); weight != 3 {
				t.Fatalf("SetEdgePropertyChecked(%T) stored %v, want 3", value, weight)
			}
		}
	}
}

type checkedWeightCase struct {
	name      string
	value     any
	effective float64
	valid     bool
}

func checkedWeightCases() []checkedWeightCase {
	return []checkedWeightCase{
		{name: "negative", value: float64(-2)},
		{name: "zero-default", value: float64(0), effective: 1, valid: true},
		{name: "positive-one", value: float64(1), effective: 1, valid: true},
		{name: "positive", value: float64(2.5), effective: 2.5, valid: true},
		{name: "positive-integer", value: int32(3), effective: 3, valid: true},
		{name: "nan", value: math.NaN()},
		{name: "positive-infinity", value: math.Inf(1)},
		{name: "negative-infinity", value: math.Inf(-1)},
		{name: "nonnumeric", value: "weight"},
		{name: "boolean-is-not-numeric", value: true},
	}
}

func checkedMutationModelNumber(value any) (float64, bool) {
	switch typed := value.(type) {
	case int32:
		return float64(typed), true
	case float64:
		return typed, true
	default:
		return 0, false
	}
}

func TestCheckedEdgeMutationsMatchWeightClassModel(t *testing.T) {
	// The reference model classifies the supplied value independently from the
	// graph APIs: only numeric finite values >= 0 are accepted, with zero->one.
	for _, repr := range representations() {
		for _, testCase := range checkedWeightCases() {
			t.Run(string(repr)+"/"+testCase.name, func(t *testing.T) {
				g := New(repr)
				weight, numeric := checkedMutationModelNumber(testCase.value)
				modelValid := numeric && !math.IsNaN(weight) && !math.IsInf(weight, 0) && weight >= 0
				if modelValid && weight == 0 {
					weight = 1
				}
				if modelValid != testCase.valid || (modelValid && weight != testCase.effective) {
					t.Fatalf("bad independent model for %s: valid=%v weight=%v", testCase.name, modelValid, weight)
				}

				addWeight, addNumeric := checkedMutationModelNumber(testCase.value)
				if !addNumeric {
					addWeight = 1
				}
				addErr := g.AddEdgeChecked("a", "b", addWeight)
				addModelValid := !math.IsNaN(addWeight) && !math.IsInf(addWeight, 0) && addWeight >= 0
				if addModelValid && addWeight == 0 {
					addWeight = 1
				}
				if addModelValid {
					if addErr != nil || g.Size() != 2 || !g.HasEdge("a", "b") {
						t.Fatalf("valid checked add state: err=%v nodes=%v edges=%v", addErr, g.Nodes(), g.Edges())
					}
					stored, _ := g.EdgeWeight("a", "b")
					if stored != addWeight {
						t.Fatalf("checked add stored %v, want %v", stored, addWeight)
					}
				} else if addErr == nil || g.Size() != 0 || len(g.Edges()) != 0 {
					t.Fatalf("invalid checked add mutated graph: err=%v nodes=%v edges=%v", addErr, g.Nodes(), g.Edges())
				}

				propertyGraph := New(repr)
				propertyGraph.AddEdge("a", "b", 5, PropertyBag{"label": "seed"})
				beforeProperties, _ := propertyGraph.EdgeProperties("a", "b")
				setErr := propertyGraph.SetEdgePropertyChecked("a", "b", "weight", testCase.value)
				if modelValid {
					if setErr != nil {
						t.Fatalf("valid checked property update: %v", setErr)
					}
					stored, _ := propertyGraph.EdgeWeight("a", "b")
					if stored != testCase.effective {
						t.Fatalf("checked property stored %v, want %v", stored, testCase.effective)
					}
				} else {
					afterProperties, _ := propertyGraph.EdgeProperties("a", "b")
					if setErr == nil || !reflect.DeepEqual(beforeProperties, afterProperties) {
						t.Fatalf("invalid property update mutated graph: err=%v before=%#v after=%#v", setErr, beforeProperties, afterProperties)
					}
				}
			})
		}
	}
}

func TestCheckedMutationAPIsPreserveLegacySignedMSTBehavior(t *testing.T) {
	for _, repr := range representations() {
		g := New(repr)
		g.AddEdge("a", "b", -2)
		g.AddEdge("b", "c", 3)
		if weight, _ := g.EdgeWeight("a", "b"); weight != -2 {
			t.Fatalf("legacy AddEdge lost finite negative weight for repr=%s: %v", repr, weight)
		}
		if err := g.SetEdgeProperty("a", "b", "weight", -4); err != nil {
			t.Fatalf("legacy SetEdgeProperty changed behavior for repr=%s: %v", repr, err)
		}
		if _, err := MinimumSpanningTree(g); err != nil {
			t.Fatalf("legacy signed finite MST failed for repr=%s: %v", repr, err)
		}
	}
}
