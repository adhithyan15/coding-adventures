// Aggregate function evaluation for the SQL execution engine.
//
// Aggregate functions collapse a set of rows into a single value:
//
//	COUNT(*) → count of all rows
//	COUNT(expr) → count of non-NULL values
//	SUM(expr) → sum of non-NULL numeric values
//	AVG(expr) → arithmetic mean of non-NULL numeric values
//	MIN(expr) → minimum non-NULL value
//	MAX(expr) → maximum non-NULL value
//
// # When aggregates apply
//
// A query uses aggregation in two scenarios:
//  1. GROUP BY: rows are partitioned by one or more key expressions.
//     Each group produces one output row. Aggregate functions operate on
//     each group independently.
//  2. No GROUP BY + aggregate in SELECT: all rows form a single implicit
//     group. The query returns exactly one row.
//
// # NULL semantics
//
// SQL aggregates ignore NULL values (except COUNT(*) which counts rows
// regardless of their content). This matches standard SQL:
//
//	SUM(NULL, NULL, 3) = 3  (not NULL)
//	AVG(NULL, 5, 10)  = 7.5 (only 5 and 10 contribute)
//	COUNT(NULL, 5)    = 1   (NULL is not counted)
//	COUNT(*)          = 2   (counts rows, not values)
//
// # Numeric coercion
//
// SUM and AVG require numeric operands. Values in rowCtx come from the
// DataSource as int64, float64, string, or nil. Non-numeric values are
// skipped with a coercion attempt: int64 is used as-is, float64 is used
// as-is, strings are skipped (we don't do implicit string→number casts).
package sqlengine

import (
	"math"

	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// aggregateResult holds the accumulated state for one aggregate function
// over a group of rows. After processing all rows, toValue() produces the
// final SQL value.
type aggregateResult struct {
	fn       string      // "COUNT", "SUM", "AVG", "MIN", "MAX"
	count    int64       // rows processed (COUNT(*)) or non-NULL values seen
	sum      float64     // running sum (SUM, AVG)
	minVal   interface{} // current minimum (nil = not yet set)
	maxVal   interface{} // current maximum (nil = not yet set)
	isStar   bool        // true for COUNT(*)
	hasValue bool        // true once at least one non-NULL value contributed
}

// computeAggregate evaluates an aggregate function over a slice of rows.
//
// Parameters:
//   - fn: "COUNT", "SUM", "AVG", "MIN", "MAX" (uppercase)
//   - argNode: the argument ASTNode inside the function call (nil for COUNT(*))
//   - isStar: true when the argument is STAR (COUNT(*))
//   - rows: the group of rows to aggregate
//   - colMap: column name → column index mapping for context building
//
// Returns the final SQL value: nil | int64 | float64.
func computeAggregate(fn string, argNode *parser.ASTNode, isStar bool, rows []map[string]interface{}) interface{} {
	acc := &aggregateResult{fn: fn, isStar: isStar}

	for _, row := range rows {
		if isStar {
			// COUNT(*) counts every row, even those with all-NULL columns.
			acc.count++
			continue
		}
		// Evaluate the argument expression for this row.
		val := evalExpr(argNode, row)
		acc.accumulate(val)
	}

	return acc.toValue()
}

// accumulate adds one value to the aggregate state.
// NULL values are ignored for COUNT(expr), SUM, AVG, MIN, MAX.
func (a *aggregateResult) accumulate(val interface{}) {
	if val == nil {
		// NULL is skipped for all aggregate functions except COUNT(*),
		// which is handled separately in computeAggregate.
		return
	}

	switch a.fn {
	case "COUNT":
		// COUNT(expr) counts non-NULL values.
		a.count++

	case "SUM", "AVG":
		// Convert to float64 for summation. int64 is lossless for values
		// up to 2^53 (about 9 quadrillion), which covers all practical cases.
		n, ok := toFloat64(val)
		if !ok {
			return // skip non-numeric values
		}
		a.sum += n
		a.count++
		a.hasValue = true

	case "MIN":
		if !a.hasValue {
			a.minVal = val
			a.hasValue = true
		} else if compareValues(val, a.minVal) < 0 {
			a.minVal = val
		}

	case "MAX":
		if !a.hasValue {
			a.maxVal = val
			a.hasValue = true
		} else if compareValues(val, a.maxVal) > 0 {
			a.maxVal = val
		}
	}
}

// toValue converts the accumulated state to the final SQL value.
func (a *aggregateResult) toValue() interface{} {
	switch a.fn {
	case "COUNT":
		return a.count // always int64, never NULL

	case "SUM":
		if !a.hasValue {
			return nil // SUM over zero non-NULL values is NULL
		}
		// Return int64 if the sum is a whole number, float64 otherwise.
		// This matches PostgreSQL's behavior for integer columns.
		if a.sum == math.Trunc(a.sum) {
			return int64(a.sum)
		}
		return a.sum

	case "AVG":
		if a.count == 0 {
			return nil // AVG over zero non-NULL values is NULL
		}
		return a.sum / float64(a.count)

	case "MIN":
		return a.minVal // nil if no non-NULL values

	case "MAX":
		return a.maxVal // nil if no non-NULL values
	}

	return nil
}

// toFloat64 converts a SQL value to float64 for arithmetic.
// Returns (value, true) for numeric types, (0, false) otherwise.
func toFloat64(v interface{}) (float64, bool) {
	switch n := v.(type) {
	case int64:
		return float64(n), true
	case float64:
		return n, true
	case int:
		return float64(n), true
	case int32:
		return float64(n), true
	}
	return 0, false
}

// compareValues compares two SQL values for MIN/MAX ordering.
// Returns negative if a < b, zero if a == b, positive if a > b.
//
// SQL comparison rules:
//   - Numbers (int64/float64) compare numerically (with cross-type promotion)
//   - Strings compare lexicographically
//   - Mixed types (number vs string) fall back to string comparison
func compareValues(a, b interface{}) int {
	// Numeric comparison: promote both to float64.
	af, aOk := toFloat64(a)
	bf, bOk := toFloat64(b)
	if aOk && bOk {
		if af < bf {
			return -1
		} else if af > bf {
			return 1
		}
		return 0
	}

	// String comparison.
	as, aStr := a.(string)
	bs, bStr := b.(string)
	if aStr && bStr {
		if as < bs {
			return -1
		} else if as > bs {
			return 1
		}
		return 0
	}

	// Boolean comparison (false < true).
	ab, aBool := a.(bool)
	bb, bBool := b.(bool)
	if aBool && bBool {
		if !ab && bb {
			return -1
		} else if ab && !bb {
			return 1
		}
		return 0
	}

	// Fallback: convert to strings and compare lexicographically.
	// This handles mixed types gracefully without panicking.
	return 0
}

// isAggregateFunction returns true if name is a known SQL aggregate function.
// These are the five standard SQL aggregate functions that every SQL database
// must support.
func isAggregateFunction(name string) bool {
	switch name {
	case "COUNT", "SUM", "AVG", "MIN", "MAX":
		return true
	}
	return false
}
