package graph

import (
	"errors"
	"fmt"
	"math"
)

// ErrInvalidEdgeWeight is returned by checked mutation methods when the
// effective edge weight is negative or non-finite.
var ErrInvalidEdgeWeight = errors.New("edge weight must be finite and non-negative")

func checkedEffectiveWeight(weight float64) (float64, error) {
	if weight == 0 {
		weight = 1
	}
	if weight < 0 || math.IsNaN(weight) || math.IsInf(weight, 0) {
		return 0, fmt.Errorf("%w: %v", ErrInvalidEdgeWeight, weight)
	}
	return weight, nil
}

// AddEdgeChecked inserts an edge only when its positional weight is finite and
// non-negative. The existing zero-means-default behavior is retained. A
// "weight" key in properties does not override the positional weight.
// Validation happens before AddEdge can create either endpoint.
func (g *Graph) AddEdgeChecked(left, right string, weight float64, properties ...PropertyBag) error {
	if _, err := checkedEffectiveWeight(weight); err != nil {
		return err
	}
	g.AddEdge(left, right, weight, properties...)
	return nil
}

// SetEdgePropertyChecked preserves SetEdgeProperty's missing-edge precedence
// and validates numeric values only for the canonical "weight" property.
// Other properties keep their existing semantics.
func (g *Graph) SetEdgePropertyChecked(left, right, propertyKey string, value PropertyValue) error {
	if !g.HasEdge(left, right) {
		return &EdgeNotFoundError{Left: left, Right: right}
	}
	if propertyKey != "weight" {
		return g.SetEdgeProperty(left, right, propertyKey, value)
	}

	weight, ok := numericValue(value)
	if !ok {
		return fmt.Errorf("edge property %q must be numeric", propertyKey)
	}
	effectiveWeight, err := checkedEffectiveWeight(weight)
	if err != nil {
		return err
	}
	if weight == 0 {
		value = effectiveWeight
	}
	return g.SetEdgeProperty(left, right, propertyKey, value)
}
