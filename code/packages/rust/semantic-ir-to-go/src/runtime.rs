//! Inlined Go runtime helpers — pasted verbatim into every artifact.
//!
//! Imports of `fmt`, `math`, and `strconv` are required by the runtime;
//! the emitter always emits them in the file header.  They are always
//! used (the runtime block as a whole references all three — `math` via
//! the SIR16 float `NaN`/`Inf` checks in `_sir_format_float`), so the Go
//! "unused import" rule is satisfied for every generated file.

pub const RUNTIME: &str = r##"// ── inlined SIR runtime ────────────────────────────────────────
type Value interface{}

type Symbol struct {
	Name string
}

type Pair struct {
	Car Value
	Cdr Value
}

type Closure struct {
	Fn func(args []Value) Value
}

var _sir_symbol_table = make(map[string]*Symbol)
var _sir_globals = make(map[string]Value)

func _sir_intern(name string) Value {
	if s, ok := _sir_symbol_table[name]; ok {
		return s
	}
	s := &Symbol{Name: name}
	_sir_symbol_table[name] = s
	return s
}

func _sir_truthy(v Value) bool {
	if v == nil {
		return false
	}
	if b, ok := v.(bool); ok {
		return b
	}
	return true
}

func _sir_apply(c Value, args []Value) Value {
	cl, ok := c.(*Closure)
	if !ok {
		panic("apply on non-closure")
	}
	return cl.Fn(args)
}

func _sir_make_closure(fn func(args []Value) Value, captures []Value) Value {
	// fn is the synthesised lambda function; it accepts a single
	// flat []Value argument expected as captures-first then params.
	// Combine the closure-time captures with the runtime args.
	return &Closure{Fn: func(args []Value) Value {
		combined := make([]Value, 0, len(captures)+len(args))
		combined = append(combined, captures...)
		combined = append(combined, args...)
		return fn(combined)
	}}
}

func _sir_global_set(name Value, value Value) Value {
	key := _sir_value_to_key(name)
	_sir_globals[key] = value
	return value
}

func _sir_global_get(name Value) Value {
	key := _sir_value_to_key(name)
	v, ok := _sir_globals[key]
	if !ok {
		panic("undefined global: " + key)
	}
	return v
}

func _sir_global_get_static(name string) Value {
	v, ok := _sir_globals[name]
	if !ok {
		panic("undefined global: " + name)
	}
	return v
}

func _sir_value_to_key(v Value) string {
	if s, ok := v.(*Symbol); ok {
		return s.Name
	}
	if s, ok := v.(string); ok {
		return s
	}
	return _sir_format(v)
}

func _sir_as_int(v Value) int64 {
	if n, ok := v.(int64); ok {
		return n
	}
	if n, ok := v.(int); ok {
		return int64(n)
	}
	panic("expected int")
}

// ── numeric tower (SIR16 floats) ───────────────────────────────
//
// `Value` gains a `float64` arm.  Arithmetic stays on the integer
// fast-path while EVERY operand is an integer (preserving exact int64
// semantics, including the `*` wrapping below).  The moment ANY operand
// is a float the whole fold promotes to float64 — matching the
// "int op float ⇒ float" rule of Python/Ruby/JS.

func _sir_is_float_val(v Value) bool {
	_, ok := v.(float64)
	return ok
}

func _sir_any_float(args []Value) bool {
	for _, a := range args {
		if _sir_is_float_val(a) {
			return true
		}
	}
	return false
}

// Coerce any number to float64 for the promoted arithmetic/comparison
// paths.  Integers widen losslessly for magnitudes within ±2^53; beyond
// that the all-integer fast paths keep exactness, so this widening only
// runs once a float is genuinely in play.
func _sir_as_float(v Value) float64 {
	switch n := v.(type) {
	case float64:
		return n
	case int64:
		return float64(n)
	case int:
		return float64(n)
	}
	panic("expected number")
}

func _sir_plus(args []Value) Value {
	if _sir_any_float(args) {
		var total float64
		for _, a := range args {
			total += _sir_as_float(a)
		}
		return total
	}
	var total int64
	for _, a := range args {
		total += _sir_as_int(a)
	}
	return total
}

func _sir_minus(args []Value) Value {
	if len(args) == 0 {
		return int64(0)
	}
	if _sir_any_float(args) {
		if len(args) == 1 {
			return -_sir_as_float(args[0])
		}
		acc := _sir_as_float(args[0])
		for _, a := range args[1:] {
			acc -= _sir_as_float(a)
		}
		return acc
	}
	if len(args) == 1 {
		return -_sir_as_int(args[0])
	}
	acc := _sir_as_int(args[0])
	for _, a := range args[1:] {
		acc -= _sir_as_int(a)
	}
	return acc
}

func _sir_times(args []Value) Value {
	if _sir_any_float(args) {
		acc := 1.0
		for _, a := range args {
			acc *= _sir_as_float(a)
		}
		return acc
	}
	var acc int64 = 1
	for _, a := range args {
		acc *= _sir_as_int(a)
	}
	return acc
}

func _sir_divide(args []Value) Value {
	if len(args) == 0 {
		return int64(0)
	}
	if _sir_any_float(args) {
		// Float division follows IEEE-754: `1.0 / 0.0` is `+Inf`
		// rather than a panic.  Only the all-integer path keeps the
		// historical divide-by-zero panic.
		acc := _sir_as_float(args[0])
		for _, a := range args[1:] {
			acc /= _sir_as_float(a)
		}
		return acc
	}
	acc := _sir_as_int(args[0])
	for _, a := range args[1:] {
		d := _sir_as_int(a)
		if d == 0 {
			panic("division by zero")
		}
		acc /= d
	}
	return acc
}

func _sir_eq(args []Value) Value {
	if len(args) < 2 {
		return true
	}
	a := args[0]
	b := args[1]
	if as, ok := a.(*Symbol); ok {
		if bs, ok := b.(*Symbol); ok {
			return as.Name == bs.Name
		}
	}
	// Cross-representation numeric equality (`1 == 1.0`) holds,
	// mirroring dynamic-language `==`.  Float/Float uses IEEE
	// equality, so `NaN == NaN` is correctly `false`.  We route ANY
	// number pair through float64 comparison; non-numbers fall back to
	// Go's `==`.
	if _sir_is_number_val(a) && _sir_is_number_val(b) {
		return _sir_as_float(a) == _sir_as_float(b)
	}
	return a == b
}

func _sir_is_number_val(v Value) bool {
	switch v.(type) {
	case int64, int, float64:
		return true
	}
	return false
}

func _sir_lt(args []Value) Value {
	if ai, aok := args[0].(int64); aok {
		if bi, bok := args[1].(int64); bok {
			return ai < bi
		}
	}
	return _sir_as_float(args[0]) < _sir_as_float(args[1])
}

func _sir_gt(args []Value) Value {
	if ai, aok := args[0].(int64); aok {
		if bi, bok := args[1].(int64); bok {
			return ai > bi
		}
	}
	return _sir_as_float(args[0]) > _sir_as_float(args[1])
}

func _sir_cons(args []Value) Value {
	return &Pair{Car: args[0], Cdr: args[1]}
}

func _sir_car(args []Value) Value {
	if p, ok := args[0].(*Pair); ok {
		return p.Car
	}
	panic("car on non-pair")
}

func _sir_cdr(args []Value) Value {
	if p, ok := args[0].(*Pair); ok {
		return p.Cdr
	}
	panic("cdr on non-pair")
}

func _sir_is_null(args []Value) Value { return args[0] == nil }

func _sir_is_pair(args []Value) Value {
	_, ok := args[0].(*Pair)
	return ok
}

func _sir_is_number(args []Value) Value {
	// `number?` names the whole numeric tower — true for integers AND
	// floats, not a single representation.
	switch args[0].(type) {
	case int64, int, float64:
		return true
	}
	return false
}

func _sir_is_symbol(args []Value) Value {
	_, ok := args[0].(*Symbol)
	return ok
}

func _sir_print(args []Value) Value {
	fmt.Println(_sir_format(args[0]))
	return nil
}

func _sir_format(v Value) string {
	if v == nil {
		return "nil"
	}
	if b, ok := v.(bool); ok {
		if b {
			return "#t"
		}
		return "#f"
	}
	if n, ok := v.(int64); ok {
		return strconv.FormatInt(n, 10)
	}
	if n, ok := v.(int); ok {
		return strconv.FormatInt(int64(n), 10)
	}
	if x, ok := v.(float64); ok {
		return _sir_format_float(x)
	}
	if s, ok := v.(string); ok {
		return s
	}
	if s, ok := v.(*Symbol); ok {
		return s.Name
	}
	if p, ok := v.(*Pair); ok {
		return _sir_format_pair(p)
	}
	if _, ok := v.(*Closure); ok {
		return "<closure>"
	}
	return fmt.Sprintf("%v", v)
}

// Render a float so integral values keep a trailing `.0` (`3.0`, not
// `3`) — making the printed form unambiguously a float, matching how
// Python/Ruby and the Rust backend's `{:?}` render `3.0`.  Non-finite
// values print as `NaN` / `inf` / `-inf` (mirroring the Rust backend).
//
//   | value | output |
//   |-------|--------|
//   | 3.0   | "3.0"  |  ← FormatFloat gives "3"; we append ".0"
//   | 3.25  | "3.25" |  ← already has a "."
//   | -7.5  | "-7.5" |
//   | 1e20  | "1e+20"|  ← exponent form already unambiguous
//   | NaN   | "NaN"  |
//   | +Inf  | "inf"  |
//   | -Inf  | "-inf" |
func _sir_format_float(x float64) string {
	if math.IsNaN(x) {
		return "NaN"
	}
	if math.IsInf(x, 1) {
		return "inf"
	}
	if math.IsInf(x, -1) {
		return "-inf"
	}
	s := strconv.FormatFloat(x, 'g', -1, 64)
	// If the shortest representation has no decimal point and no
	// exponent, it looks like an integer — append ".0" to keep the
	// float identity visible.
	for i := 0; i < len(s); i++ {
		c := s[i]
		if c == '.' || c == 'e' || c == 'E' {
			return s
		}
	}
	return s + ".0"
}

func _sir_format_pair(p *Pair) string {
	out := "(" + _sir_format(p.Car)
	rest := p.Cdr
	for {
		if next, ok := rest.(*Pair); ok {
			out += " " + _sir_format(next.Car)
			rest = next.Cdr
			continue
		}
		if rest == nil {
			break
		}
		out += " . " + _sir_format(rest)
		break
	}
	return out + ")"
}

// ── SIR16 loops: ForRange continue test ────────────────────────
//
// Direction-aware half-open bound check for a `ForRange` counter.  With
// a non-negative `step` the loop runs while `i < stop` (counting up);
// with a negative `step` it runs while `i > stop` (counting down).  This
// keeps the emitted three-clause `for` header readable while preserving
// Python's `range` semantics for negative steps.
func _sir_range_cont(i int64, stop int64, step int64) bool {
	if step >= 0 {
		return i < stop
	}
	return i > stop
}

// ── SIR16 loops: cons-list iteration (ForEach) ─────────────────
//
// `ForEach` iterates a "sequence" value.  This backend has no dedicated
// `Seq` value yet (Sequences land in a later PR), so a sequence is the
// classic cons-list: a `Pair`-chain whose final `cdr` is `nil`.  `nil`
// itself is the empty sequence.  `_sir_seq_iter` flattens that chain
// into a `[]Value` the `for ... range` loop can walk.  An improper list
// (a non-`nil`, non-`Pair` tail) is a programming error and panics,
// matching the strictness of `car`/`cdr` on a non-pair.
func _sir_seq_iter(v Value) []Value {
	out := []Value{}
	cur := v
	for {
		if cur == nil {
			break
		}
		if p, ok := cur.(*Pair); ok {
			out = append(out, p.Car)
			cur = p.Cdr
			continue
		}
		panic("cannot iterate non-sequence: " + _sir_format(cur))
	}
	return out
}

func _sir_builtin_closure(name string) Value {
	return &Closure{Fn: func(args []Value) Value {
		return _sir_call_builtin_by_name(name, args)
	}}
}

func _sir_call_builtin_by_name(name string, args []Value) Value {
	switch name {
	case "+":
		return _sir_plus(args)
	case "-":
		return _sir_minus(args)
	case "*":
		return _sir_times(args)
	case "/":
		return _sir_divide(args)
	case "=":
		return _sir_eq(args)
	case "<":
		return _sir_lt(args)
	case ">":
		return _sir_gt(args)
	case "cons":
		return _sir_cons(args)
	case "car":
		return _sir_car(args)
	case "cdr":
		return _sir_cdr(args)
	case "null?":
		return _sir_is_null(args)
	case "pair?":
		return _sir_is_pair(args)
	case "number?":
		return _sir_is_number(args)
	case "symbol?":
		return _sir_is_symbol(args)
	case "print":
		return _sir_print(args)
	case "global_set":
		return _sir_global_set(args[0], args[1])
	case "global_get":
		return _sir_global_get(args[0])
	}
	panic("unknown builtin: " + name)
}

"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_non_empty_ends_newline() {
        assert!(!RUNTIME.is_empty());
        assert!(RUNTIME.ends_with('\n'));
    }

    #[test]
    fn runtime_uses_fmt_and_strconv() {
        // The emitter always emits `import ("fmt"; "math"; "strconv")`
        // so all three must be referenced in the runtime to satisfy
        // Go's unused-import rule.
        assert!(RUNTIME.contains("fmt.Println"));
        assert!(RUNTIME.contains("fmt.Sprintf"));
        assert!(RUNTIME.contains("strconv.FormatInt"));
        assert!(RUNTIME.contains("strconv.FormatFloat"));
        assert!(RUNTIME.contains("math.IsNaN"));
        assert!(RUNTIME.contains("math.IsInf"));
    }

    #[test]
    fn runtime_declares_float_helpers() {
        // SIR16 floats: the value model accepts a `float64` arm, and
        // the numeric helpers gain float coercion + a display path.
        assert!(RUNTIME.contains("_sir_as_float"));
        assert!(RUNTIME.contains("_sir_any_float"));
        assert!(RUNTIME.contains("_sir_format_float"));
        assert!(RUNTIME.contains("_sir_is_number_val"));
    }

    #[test]
    fn runtime_declares_loop_helpers() {
        // SIR16 Loops: ForRange needs `_sir_as_int` (already present for
        // floats) + the direction-aware `_sir_range_cont`; ForEach needs
        // the cons-list flattener `_sir_seq_iter`.
        assert!(RUNTIME.contains("func _sir_range_cont(i int64, stop int64, step int64) bool"));
        assert!(RUNTIME.contains("func _sir_seq_iter(v Value) []Value"));
        assert!(RUNTIME.contains("_sir_as_int"));
    }

    #[test]
    fn runtime_declares_value_types() {
        assert!(RUNTIME.contains("type Value interface{}"));
        assert!(RUNTIME.contains("type Symbol struct"));
        assert!(RUNTIME.contains("type Pair struct"));
        assert!(RUNTIME.contains("type Closure struct"));
    }

    #[test]
    fn runtime_includes_all_builtins() {
        for s in &[
            "_sir_plus", "_sir_minus", "_sir_times", "_sir_divide",
            "_sir_eq", "_sir_lt", "_sir_gt",
            "_sir_cons", "_sir_car", "_sir_cdr",
            "_sir_is_null", "_sir_is_pair", "_sir_is_number", "_sir_is_symbol",
            "_sir_print", "_sir_global_set", "_sir_global_get",
            "_sir_apply", "_sir_make_closure", "_sir_intern", "_sir_truthy",
            "_sir_format", "_sir_builtin_closure", "_sir_call_builtin_by_name",
        ] {
            assert!(RUNTIME.contains(s), "missing: {}", s);
        }
    }
}
