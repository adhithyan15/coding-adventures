package graph

import (
	"errors"
	"math"
	"reflect"
	"testing"
)

func checkedFuzzValue(code byte) any {
	switch code % 8 {
	case 0:
		return float64(-2)
	case 1:
		return float64(0)
	case 2:
		return float64(2.5)
	case 3:
		return int32(3)
	case 4:
		return math.NaN()
	case 5:
		return math.Inf(1)
	case 6:
		return math.Inf(-1)
	default:
		return "not-a-number"
	}
}

// FuzzCheckedEdgeMutationClasses bounds each input to one representation and
// one value class, then checks both checked mutation methods against a small
// independent finite-domain model.
func FuzzCheckedEdgeMutationClasses(f *testing.F) {
	for code := byte(0); code < 8; code++ {
		f.Add([]byte{0, code})
		f.Add([]byte{1, code})
	}

	f.Fuzz(func(t *testing.T, input []byte) {
		var encoded [2]byte
		copy(encoded[:], input)
		repr := representations()[int(encoded[0]%2)]
		value := checkedFuzzValue(encoded[1])
		weight, numeric := checkedMutationModelNumber(value)
		modelValid := numeric && !math.IsNaN(weight) && !math.IsInf(weight, 0) && weight >= 0
		if modelValid && weight == 0 {
			weight = 1
		}

		addGraph := New(repr)
		addWeight := weight
		if !numeric {
			// AddEdgeChecked accepts a float64 positional weight; the
			// nonnumeric class belongs to the property-update API.
			addWeight = 3
		}
		addModelValid := !math.IsNaN(addWeight) && !math.IsInf(addWeight, 0) && addWeight >= 0
		if addModelValid && addWeight == 0 {
			addWeight = 1
		}
		addErr := addGraph.AddEdgeChecked("left", "right", addWeight)
		if addModelValid {
			stored, edgeErr := addGraph.EdgeWeight("left", "right")
			if addErr != nil || edgeErr != nil || stored != addWeight {
				t.Fatalf("valid AddEdgeChecked input=%x repr=%s: stored=%v err=%v edgeErr=%v", input, repr, stored, addErr, edgeErr)
			}
		} else if !errors.Is(addErr, ErrInvalidEdgeWeight) || addGraph.Size() != 0 || len(addGraph.Edges()) != 0 {
			t.Fatalf("invalid AddEdgeChecked input=%x repr=%s mutated graph: err=%v nodes=%v edges=%v", input, repr, addErr, addGraph.Nodes(), addGraph.Edges())
		}

		setGraph := New(repr)
		setGraph.AddEdge("left", "right", 5, PropertyBag{"label": "seed"})
		before, err := setGraph.EdgeProperties("left", "right")
		if err != nil {
			t.Fatal(err)
		}
		setErr := setGraph.SetEdgePropertyChecked("left", "right", "weight", value)
		if modelValid {
			stored, edgeErr := setGraph.EdgeWeight("left", "right")
			if setErr != nil || edgeErr != nil || stored != weight {
				t.Fatalf("valid SetEdgePropertyChecked input=%x repr=%s: stored=%v err=%v edgeErr=%v", input, repr, stored, setErr, edgeErr)
			}
		} else {
			after, edgeErr := setGraph.EdgeProperties("left", "right")
			if setErr == nil || edgeErr != nil || !reflect.DeepEqual(before, after) {
				t.Fatalf("invalid SetEdgePropertyChecked input=%x repr=%s mutated graph: err=%v before=%#v after=%#v", input, repr, setErr, before, after)
			}
		}
	})
}
