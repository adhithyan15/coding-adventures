// builtins.go — 23 built-in functions for the Starlark virtual machine.
//
// ════════════════════════════════════════════════════════════════════════
// OVERVIEW
// ════════════════════════════════════════════════════════════════════════
//
// Built-in functions are the "standard library" of Starlark.  They are
// always available without importing anything — just like `print()`,
// `len()`, and `range()` in Python.
//
// Each builtin is registered with the GenericVM's builtin registry.
// When LOAD_NAME encounters a name that isn't in the variables map,
// it checks builtins as a fallback.  When CALL_FUNCTION encounters a
// BuiltinFunction on the stack, it calls its Implementation directly
// (no frame setup needed — builtins are "native" functions).
//
// ════════════════════════════════════════════════════════════════════════
// BUILTIN CATALOG
// ════════════════════════════════════════════════════════════════════════
//
//   Category     | Builtins
//   ─────────────┼─────────────────────────────────────────
//   Output       | print
//   Type info    | type, bool, int, float, str, list, dict, tuple
//   Collections  | len, range, sorted, reversed, enumerate, zip
//   Math         | min, max, abs
//   Logic        | all, any
//   Strings      | repr
//   Attribute    | hasattr, getattr
//
// ════════════════════════════════════════════════════════════════════════
// HOW BUILTINS DIFFER FROM USER FUNCTIONS
// ════════════════════════════════════════════════════════════════════════
//
// User functions (def ...):
//   - Stored as StarlarkFunction objects.
//   - Executed by setting up a new frame and running bytecode.
//   - Can access local variables, closures, globals.
//
// Built-in functions:
//   - Stored as BuiltinFunction objects.
//   - Executed by calling a Go function directly.
//   - No bytecode involved — pure Go implementation.
//   - Cannot be redefined by user code (they live in a separate registry).
//
// This is analogous to how CPython implements builtins: they are C
// functions wrapped in a Python callable interface, called directly
// by the VM without going through the bytecode interpreter.
//
package starlarkvm

import (
	"fmt"
	"sort"
	"strings"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// RegisterAllBuiltins registers all 23 Starlark built-in functions
// with the given GenericVM.  This is called once during VM creation.
func RegisterAllBuiltins(v *vm.GenericVM) {
	// ── Output ──────────────────────────────────────────────────────

	// print(*args) — Format arguments and return as a string.
	//
	// In a real Starlark interpreter, print() writes to stdout.
	// In our VM, the PRINT_VALUE opcode handles output capture.
	// This builtin is provided so that `print` can be called as a
	// first-class function (e.g., passed to map()).
	//
	//   print("hello", "world")  → "hello world"
	//   print(42)                → "42"
	v.RegisterBuiltin("print", func(args ...interface{}) interface{} {
		parts := make([]string, len(args))
		for i, arg := range args {
			parts[i] = formatValue(arg)
		}
		result := strings.Join(parts, " ")
		// Append to VM output directly.
		v.Output = append(v.Output, result)
		return nil
	})

	// ── Type Information ────────────────────────────────────────────

	// type(x) — Returns the type name of x as a string.
	//
	// Starlark type names follow Python conventions:
	//   type(42)       → "int"
	//   type(3.14)     → "float"
	//   type("hi")     → "string"
	//   type([1,2])    → "list"
	//   type({"a":1})  → "dict"
	//   type(True)     → "bool"
	//   type(None)     → "NoneType"
	//   type(func)     → "function"
	v.RegisterBuiltin("type", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return "NoneType"
		}
		val := args[0]
		if val == nil {
			return "NoneType"
		}
		switch val.(type) {
		case bool:
			return "bool"
		case int:
			return "int"
		case float64:
			return "float"
		case string:
			return "string"
		case []interface{}:
			return "list"
		case map[string]interface{}:
			return "dict"
		case *StarlarkFunction:
			return "function"
		case *vm.BuiltinFunction:
			return "function"
		default:
			return fmt.Sprintf("%T", val)
		}
	})

	// bool(x) — Convert x to a boolean.
	//
	// Uses Starlark's truthiness rules:
	//   bool(0)      → False
	//   bool("")     → False
	//   bool([])     → False
	//   bool(42)     → True
	//   bool("hi")   → True
	v.RegisterBuiltin("bool", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return false
		}
		return !isFalsy(args[0])
	})

	// int(x) — Convert x to an integer.
	//
	//   int(3.14)    → 3
	//   int("42")    → 42
	//   int(True)    → 1
	v.RegisterBuiltin("int", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return 0
		}
		val := args[0]
		switch v := val.(type) {
		case int:
			return v
		case float64:
			return int(v)
		case bool:
			if v {
				return 1
			}
			return 0
		case string:
			var n int
			_, err := fmt.Sscanf(v, "%d", &n)
			if err != nil {
				panic(fmt.Sprintf("ValueError: invalid literal for int(): '%s'", v))
			}
			return n
		}
		panic(fmt.Sprintf("TypeError: int() argument must be a string or number, not '%T'", val))
	})

	// float(x) — Convert x to a float.
	//
	//   float(42)    → 42.0
	//   float("3.14") → 3.14
	v.RegisterBuiltin("float", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return 0.0
		}
		val := args[0]
		switch v := val.(type) {
		case float64:
			return v
		case int:
			return float64(v)
		case bool:
			if v {
				return 1.0
			}
			return 0.0
		case string:
			var f float64
			_, err := fmt.Sscanf(v, "%f", &f)
			if err != nil {
				panic(fmt.Sprintf("ValueError: could not convert string to float: '%s'", v))
			}
			return f
		}
		panic(fmt.Sprintf("TypeError: float() argument must be a string or number, not '%T'", val))
	})

	// str(x) — Convert x to its string representation.
	//
	//   str(42)    → "42"
	//   str(True)  → "True"
	//   str(None)  → "None"
	//   str([1,2]) → "[1, 2]"
	v.RegisterBuiltin("str", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return ""
		}
		return formatValue(args[0])
	})

	// list(iterable) — Convert an iterable to a list.
	//
	//   list("abc")  → ["a", "b", "c"]
	//   list((1,2))  → [1, 2]
	//   list()       → []
	v.RegisterBuiltin("list", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return []interface{}{}
		}
		val := args[0]
		switch v := val.(type) {
		case []interface{}:
			result := make([]interface{}, len(v))
			copy(result, v)
			return result
		case string:
			result := make([]interface{}, len(v))
			for i, ch := range v {
				result[i] = string(ch)
			}
			return result
		case map[string]interface{}:
			keys := make([]interface{}, 0, len(v))
			for k := range v {
				keys = append(keys, k)
			}
			return keys
		}
		return []interface{}{val}
	})

	// dict() — Create an empty dict.
	//
	//   dict()  → {}
	v.RegisterBuiltin("dict", func(args ...interface{}) interface{} {
		return make(map[string]interface{})
	})

	// tuple(iterable) — Convert an iterable to a tuple.
	// In Go, tuples are the same as lists ([]interface{}).
	v.RegisterBuiltin("tuple", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return []interface{}{}
		}
		val := args[0]
		switch v := val.(type) {
		case []interface{}:
			result := make([]interface{}, len(v))
			copy(result, v)
			return result
		case string:
			result := make([]interface{}, len(v))
			for i, ch := range v {
				result[i] = string(ch)
			}
			return result
		}
		return []interface{}{val}
	})

	// ── Collection Operations ───────────────────────────────────────

	// len(x) — Return the length of a string, list, dict, or tuple.
	//
	//   len("hello")     → 5
	//   len([1, 2, 3])   → 3
	//   len({"a": 1})    → 1
	//   len(())          → 0
	v.RegisterBuiltin("len", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			panic("TypeError: len() takes exactly one argument (0 given)")
		}
		val := args[0]
		switch v := val.(type) {
		case string:
			return len(v)
		case []interface{}:
			return len(v)
		case map[string]interface{}:
			return len(v)
		}
		panic(fmt.Sprintf("TypeError: object of type '%T' has no len()", val))
	})

	// range(stop) or range(start, stop) or range(start, stop, step)
	//
	// Returns a list of integers.  This is Starlark's range(), which
	// eagerly produces a list (unlike Python 3's lazy range object).
	//
	//   range(5)        → [0, 1, 2, 3, 4]
	//   range(2, 5)     → [2, 3, 4]
	//   range(0, 10, 3) → [0, 3, 6, 9]
	//   range(5, 0, -1) → [5, 4, 3, 2, 1]
	v.RegisterBuiltin("range", func(args ...interface{}) interface{} {
		var start, stop, step int
		switch len(args) {
		case 1:
			start = 0
			stop = toInt(args[0])
			step = 1
		case 2:
			start = toInt(args[0])
			stop = toInt(args[1])
			step = 1
		case 3:
			start = toInt(args[0])
			stop = toInt(args[1])
			step = toInt(args[2])
		default:
			panic("TypeError: range() takes 1 to 3 arguments")
		}

		if step == 0 {
			panic("ValueError: range() step argument must not be zero")
		}

		result := []interface{}{}
		if step > 0 {
			for i := start; i < stop; i += step {
				result = append(result, i)
			}
		} else {
			for i := start; i > stop; i += step {
				result = append(result, i)
			}
		}
		return result
	})

	// sorted(iterable) — Return a new sorted list.
	//
	// Uses lexicographic string comparison for mixed types.
	//
	//   sorted([3, 1, 2])    → [1, 2, 3]
	//   sorted("cab")        → ["a", "b", "c"]
	v.RegisterBuiltin("sorted", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return []interface{}{}
		}
		val := args[0]
		var items []interface{}
		switch v := val.(type) {
		case []interface{}:
			items = make([]interface{}, len(v))
			copy(items, v)
		case string:
			items = make([]interface{}, len(v))
			for i, ch := range v {
				items[i] = string(ch)
			}
		default:
			return []interface{}{}
		}

		sort.Slice(items, func(i, j int) bool {
			a, b := items[i], items[j]
			if isNumeric(a) && isNumeric(b) {
				return toFloat(a) < toFloat(b)
			}
			return fmt.Sprintf("%v", a) < fmt.Sprintf("%v", b)
		})
		return items
	})

	// reversed(iterable) — Return a new reversed list.
	//
	//   reversed([1, 2, 3])  → [3, 2, 1]
	//   reversed("abc")      → ["c", "b", "a"]
	v.RegisterBuiltin("reversed", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return []interface{}{}
		}
		val := args[0]
		var items []interface{}
		switch v := val.(type) {
		case []interface{}:
			items = make([]interface{}, len(v))
			copy(items, v)
		case string:
			items = make([]interface{}, len(v))
			for i, ch := range v {
				items[i] = string(ch)
			}
		default:
			return []interface{}{}
		}

		// Reverse in place.
		for i, j := 0, len(items)-1; i < j; i, j = i+1, j-1 {
			items[i], items[j] = items[j], items[i]
		}
		return items
	})

	// enumerate(iterable) — Return a list of (index, item) pairs.
	//
	//   enumerate(["a", "b", "c"])  → [[0, "a"], [1, "b"], [2, "c"]]
	v.RegisterBuiltin("enumerate", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return []interface{}{}
		}
		val := args[0]
		var items []interface{}
		switch v := val.(type) {
		case []interface{}:
			items = v
		case string:
			items = make([]interface{}, len(v))
			for i, ch := range v {
				items[i] = string(ch)
			}
		default:
			return []interface{}{}
		}

		result := make([]interface{}, len(items))
		for i, item := range items {
			result[i] = []interface{}{i, item}
		}
		return result
	})

	// zip(list1, list2, ...) — Zip multiple lists into a list of tuples.
	//
	//   zip([1, 2], ["a", "b"])  → [[1, "a"], [2, "b"]]
	//
	// Stops at the shortest list.
	v.RegisterBuiltin("zip", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return []interface{}{}
		}

		// Convert all args to lists.
		lists := make([][]interface{}, len(args))
		minLen := -1
		for i, arg := range args {
			switch v := arg.(type) {
			case []interface{}:
				lists[i] = v
			default:
				lists[i] = []interface{}{}
			}
			if minLen < 0 || len(lists[i]) < minLen {
				minLen = len(lists[i])
			}
		}

		if minLen < 0 {
			minLen = 0
		}

		result := make([]interface{}, minLen)
		for i := 0; i < minLen; i++ {
			tuple := make([]interface{}, len(lists))
			for j, lst := range lists {
				tuple[j] = lst[i]
			}
			result[i] = tuple
		}
		return result
	})

	// ── Math ────────────────────────────────────────────────────────

	// min(*args) — Return the minimum value.
	//
	//   min(3, 1, 2)     → 1
	//   min([3, 1, 2])   → 1
	v.RegisterBuiltin("min", func(args ...interface{}) interface{} {
		items := flattenArgs(args)
		if len(items) == 0 {
			panic("ValueError: min() arg is an empty sequence")
		}
		result := items[0]
		for _, item := range items[1:] {
			if compareValues(item, result) < 0 {
				result = item
			}
		}
		return result
	})

	// max(*args) — Return the maximum value.
	//
	//   max(3, 1, 2)     → 3
	//   max([3, 1, 2])   → 3
	v.RegisterBuiltin("max", func(args ...interface{}) interface{} {
		items := flattenArgs(args)
		if len(items) == 0 {
			panic("ValueError: max() arg is an empty sequence")
		}
		result := items[0]
		for _, item := range items[1:] {
			if compareValues(item, result) > 0 {
				result = item
			}
		}
		return result
	})

	// abs(x) — Return the absolute value of x.
	//
	//   abs(-5)    → 5
	//   abs(3.14)  → 3.14
	//   abs(-2.5)  → 2.5
	v.RegisterBuiltin("abs", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			panic("TypeError: abs() takes exactly one argument (0 given)")
		}
		val := args[0]
		switch v := val.(type) {
		case int:
			if v < 0 {
				return -v
			}
			return v
		case float64:
			if v < 0 {
				return -v
			}
			return v
		}
		panic(fmt.Sprintf("TypeError: bad operand type for abs(): '%T'", val))
	})

	// ── Logic ───────────────────────────────────────────────────────

	// all(iterable) — Return True if all elements are truthy.
	//
	//   all([True, 1, "hi"])  → True
	//   all([True, 0, "hi"])  → False
	//   all([])               → True  (vacuous truth)
	v.RegisterBuiltin("all", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return true
		}
		items, ok := args[0].([]interface{})
		if !ok {
			return !isFalsy(args[0])
		}
		for _, item := range items {
			if isFalsy(item) {
				return false
			}
		}
		return true
	})

	// any(iterable) — Return True if any element is truthy.
	//
	//   any([False, 0, "hi"])  → True
	//   any([False, 0, ""])    → False
	//   any([])                → False
	v.RegisterBuiltin("any", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return false
		}
		items, ok := args[0].([]interface{})
		if !ok {
			return !isFalsy(args[0])
		}
		for _, item := range items {
			if !isFalsy(item) {
				return true
			}
		}
		return false
	})

	// ── Strings ─────────────────────────────────────────────────────

	// repr(x) — Return the repr-style string representation of x.
	//
	// Unlike str(), repr() puts quotes around strings:
	//   repr("hello")  → '"hello"'
	//   repr(42)       → "42"
	v.RegisterBuiltin("repr", func(args ...interface{}) interface{} {
		if len(args) == 0 {
			return "None"
		}
		return reprValue(args[0])
	})

	// ── Attribute Inspection ────────────────────────────────────────

	// hasattr(obj, name) — Check if obj has attribute name.
	//
	// For dicts, this checks if the key exists:
	//   hasattr({"a": 1}, "a")  → True
	//   hasattr({"a": 1}, "b")  → False
	v.RegisterBuiltin("hasattr", func(args ...interface{}) interface{} {
		if len(args) < 2 {
			panic("TypeError: hasattr() takes exactly 2 arguments")
		}
		obj := args[0]
		name := fmt.Sprintf("%v", args[1])
		if dict, ok := obj.(map[string]interface{}); ok {
			_, found := dict[name]
			return found
		}
		return false
	})

	// getattr(obj, name, default=None) — Get attribute from obj.
	//
	// For dicts, this gets the key value:
	//   getattr({"a": 1}, "a")         → 1
	//   getattr({"a": 1}, "b", 42)     → 42  (default)
	//   getattr({"a": 1}, "b")         → None
	v.RegisterBuiltin("getattr", func(args ...interface{}) interface{} {
		if len(args) < 2 {
			panic("TypeError: getattr() takes at least 2 arguments")
		}
		obj := args[0]
		name := fmt.Sprintf("%v", args[1])
		var defaultVal interface{}
		if len(args) >= 3 {
			defaultVal = args[2]
		}

		if dict, ok := obj.(map[string]interface{}); ok {
			if val, found := dict[name]; found {
				return val
			}
		}
		return defaultVal
	})
}

// flattenArgs is a helper for min/max that handles both:
//   min(1, 2, 3)    — multiple args
//   min([1, 2, 3])  — single iterable arg
//
// If there's exactly one argument and it's a list, return the list.
// Otherwise, return all arguments as a list.
func flattenArgs(args []interface{}) []interface{} {
	if len(args) == 1 {
		if lst, ok := args[0].([]interface{}); ok {
			return lst
		}
	}
	return args
}
