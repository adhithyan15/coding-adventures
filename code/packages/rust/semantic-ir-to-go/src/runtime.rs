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

// ── SIR16 sequences ────────────────────────────────────────────
//
// A `Seq` is a growable, *mutably shared* vector.  The pointer is the
// crux: a `Value` holds a `*Seq` (a shared handle), so `SeqSet`
// (`xs[i] = v`) mutates the very sequence the caller holds, and two
// bindings that alias the same literal see each other's writes — exactly
// the reference semantics of a Python list or JS array.  Wrapping the
// `[]Value` in a struct (rather than a bare `*[]Value`) keeps field
// access readable and lets a future PR hang metadata off the sequence.
// Copying a `Value` that holds a `*Seq` copies the pointer, not the
// backing slice.
type Seq struct {
	Items []Value
}

// ── SIR16 maps ─────────────────────────────────────────────────
//
// A `Map` is an *insertion-ordered* association list.  Go's native `map`
// can't key on an arbitrary `Value` (floats, closures, nested seqs/maps
// are not comparable / hashable the way we need), so — mirroring the
// Rust backend's choice — we key by `Value` using the runtime's own
// structural equality (`_sir_value_eq`, a linear scan) over a
// `[]MapEntry`.  This gives correct `MapGet`/`MapSet` semantics
// (including missing-key ⇒ `nil`) for *any* key type and preserves
// insertion order for deterministic iteration/printing.  Shared + mutable
// via a `*Map` pointer, same as `Seq`.
type MapEntry struct {
	Key Value
	Val Value
}

type Map struct {
	Entries []MapEntry
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
	return _sir_value_eq(args[0], args[1])
}

// Structural value-equality across the whole value tower.  This is the
// single source of truth for `=` (via `_sir_eq`) AND for map key lookup
// (`_sir_map_get`/`_sir_map_set`), so a float, string, symbol, or even a
// nested seq/map can be a map key with the same semantics as `=`.
//
//   | a, b              | rule                                        |
//   |-------------------|---------------------------------------------|
//   | both numbers      | compare as float64 (`1 == 1.0`; `NaN != NaN`)|
//   | both symbols      | intern-name equality                        |
//   | both pairs        | structural (car & cdr recursively)          |
//   | both seqs         | same handle, or element-wise equal          |
//   | both maps         | same handle, or entry-wise equal (in order) |
//   | otherwise         | Go `==` (bool/nil/string/closure identity)  |
func _sir_value_eq(a Value, b Value) bool {
	if as, ok := a.(*Symbol); ok {
		if bs, ok := b.(*Symbol); ok {
			return as.Name == bs.Name
		}
		return false
	}
	// Cross-representation numeric equality (`1 == 1.0`) holds,
	// mirroring dynamic-language `==`.  Float/Float uses IEEE
	// equality, so `NaN == NaN` is correctly `false`.
	if _sir_is_number_val(a) && _sir_is_number_val(b) {
		return _sir_as_float(a) == _sir_as_float(b)
	}
	if ap, ok := a.(*Pair); ok {
		if bp, ok := b.(*Pair); ok {
			return _sir_value_eq(ap.Car, bp.Car) && _sir_value_eq(ap.Cdr, bp.Cdr)
		}
		return false
	}
	// Sequences and maps compare *structurally* (element-wise / entry-
	// wise), matching how pairs compare.  Identical handles short-circuit
	// without a deep walk.  Maps compare in insertion order, which is
	// sufficient because `_sir_map_lit`/`_sir_map_set` keep a canonical
	// first-seen order — equal maps built the same way share it.
	if as, ok := a.(*Seq); ok {
		bs, ok := b.(*Seq)
		if !ok {
			return false
		}
		if as == bs {
			return true
		}
		if len(as.Items) != len(bs.Items) {
			return false
		}
		for i := range as.Items {
			if !_sir_value_eq(as.Items[i], bs.Items[i]) {
				return false
			}
		}
		return true
	}
	if am, ok := a.(*Map); ok {
		bm, ok := b.(*Map)
		if !ok {
			return false
		}
		if am == bm {
			return true
		}
		if len(am.Entries) != len(bm.Entries) {
			return false
		}
		for i := range am.Entries {
			if !_sir_value_eq(am.Entries[i].Key, bm.Entries[i].Key) ||
				!_sir_value_eq(am.Entries[i].Val, bm.Entries[i].Val) {
				return false
			}
		}
		return true
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
	if s, ok := v.(*Seq); ok {
		return _sir_format_seq(s)
	}
	if m, ok := v.(*Map); ok {
		return _sir_format_map(m)
	}
	if _, ok := v.(*Closure); ok {
		return "<closure>"
	}
	return fmt.Sprintf("%v", v)
}

// Sequences print like a bracketed list: `[1, 2, 3]`.
func _sir_format_seq(s *Seq) string {
	out := "["
	for i, item := range s.Items {
		if i > 0 {
			out += ", "
		}
		out += _sir_format(item)
	}
	return out + "]"
}

// Maps print like a brace-wrapped entry list in insertion order:
// `{a: 1, b: 2}`.
func _sir_format_map(m *Map) string {
	out := "{"
	for i, e := range m.Entries {
		if i > 0 {
			out += ", "
		}
		out += _sir_format(e.Key) + ": " + _sir_format(e.Val)
	}
	return out + "}"
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

// ── SIR16 loops: sequence iteration (ForEach) ──────────────────
//
// `ForEach` iterates a "sequence" value.  SIR16 introduced two distinct
// "sequence" shapes this backend must iterate uniformly:
//
//   * `*Seq` — the real `Sequences` value (a `SeqLit`, `[1, 2, 3]`).  We
//     snapshot its current elements into a fresh `[]Value` so the loop
//     body sees a stable view even if it mutates the underlying sequence.
//   * the classic cons-list — a `Pair`-chain whose final `cdr` is `nil`
//     (what `cons`/`car`/`cdr` build).  `nil` itself is the empty
//     sequence.
//
// Keeping both keeps the A5 `ForEach`-over-cons-list working while making
// `for x in [1, 2, 3]` (a `SeqLit`) iterate end to end.  An improper list
// (a non-`nil`, non-`Pair` tail) is a programming error and panics,
// matching the strictness of `car`/`cdr` on a non-pair.
func _sir_seq_iter(v Value) []Value {
	// A real sequence: snapshot its current elements.
	if s, ok := v.(*Seq); ok {
		out := make([]Value, len(s.Items))
		copy(out, s.Items)
		return out
	}
	// Otherwise treat it as a cons-list.
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

// ── SIR16 sequence ops (Sequences) ─────────────────────────────
//
// A `*Seq` wraps a shared, mutable `[]Value`.  These helpers are the
// lowering targets for `SeqLit`/`SeqIndex`/`SeqLen`/`SeqSet`.

// `_sir_seq_lit([a, b, ...])` constructs a fresh sequence from its items.
func _sir_seq_lit(items []Value) Value {
	return &Seq{Items: items}
}

// `_sir_seq_index(seq, i)` reads `seq[i]`.  The index is taken as an
// integer; a negative or out-of-range index panics (sequences are
// strict, like `car`/`cdr`) — we define out-of-bounds as a panic.
func _sir_seq_index(seq Value, index Value) Value {
	s, ok := seq.(*Seq)
	if !ok {
		panic("seq-index on non-sequence: " + _sir_format(seq))
	}
	i := _sir_as_int(index)
	if i < 0 || int(i) >= len(s.Items) {
		panic("sequence index out of range: " + strconv.FormatInt(i, 10))
	}
	return s.Items[i]
}

// `_sir_seq_len(seq)` returns the element count as an `int64`.
func _sir_seq_len(seq Value) Value {
	s, ok := seq.(*Seq)
	if !ok {
		panic("seq-len on non-sequence: " + _sir_format(seq))
	}
	return int64(len(s.Items))
}

// `_sir_seq_set(seq, i, value)` writes `seq[i] = value`, mutating the
// shared backing slice in place.  Out-of-range writes panic (we do not
// auto-grow, matching the index read's strictness).  Returns the written
// value so the emitter can use it in expression position if needed.
func _sir_seq_set(seq Value, index Value, value Value) Value {
	s, ok := seq.(*Seq)
	if !ok {
		panic("seq-set on non-sequence: " + _sir_format(seq))
	}
	i := _sir_as_int(index)
	if i < 0 || int(i) >= len(s.Items) {
		panic("sequence index out of range: " + strconv.FormatInt(i, 10))
	}
	s.Items[i] = value
	return value
}

// ── SIR16 map ops (Maps) ───────────────────────────────────────
//
// A `*Map` wraps a shared, mutable, insertion-ordered `[]MapEntry`.
// Lookups use `_sir_value_eq` for key comparison, so any value type
// (including a float, string, or symbol) can be a key with the same
// structural-equality semantics as `=`.

// `_sir_map_lit([(k0, v0), ...])` builds a fresh map.  A later entry with
// a key equal to an earlier one overwrites in place, so the literal
// `{a: 1, a: 2}` yields `{a: 2}` (last-write-wins) while keeping
// first-seen insertion order.
func _sir_map_lit(keys []Value, vals []Value) Value {
	m := &Map{Entries: make([]MapEntry, 0, len(keys))}
	for i := range keys {
		_sir_map_put(m, keys[i], vals[i])
	}
	return m
}

// `_sir_map_get(map, key)` reads `map[key]`, returning the associated
// value or `nil` when the key is absent (we choose `nil` for the
// target-defined missing-key behaviour, mirroring the other backends).
func _sir_map_get(mp Value, key Value) Value {
	m, ok := mp.(*Map)
	if !ok {
		panic("map-get on non-map: " + _sir_format(mp))
	}
	for i := range m.Entries {
		if _sir_value_eq(m.Entries[i].Key, key) {
			return m.Entries[i].Val
		}
	}
	return nil
}

// `_sir_map_set(map, key, value)` inserts or overwrites `map[key]`,
// mutating the shared backing store.  Returns the written value.
func _sir_map_set(mp Value, key Value, value Value) Value {
	m, ok := mp.(*Map)
	if !ok {
		panic("map-set on non-map: " + _sir_format(mp))
	}
	_sir_map_put(m, key, value)
	return value
}

// Shared insert-or-overwrite for `_sir_map_lit`/`_sir_map_set`: a new key
// appends (preserving insertion order); an existing key (by
// `_sir_value_eq`) overwrites in place without disturbing order.
func _sir_map_put(m *Map, key Value, value Value) {
	for i := range m.Entries {
		if _sir_value_eq(m.Entries[i].Key, key) {
			m.Entries[i].Val = value
			return
		}
	}
	m.Entries = append(m.Entries, MapEntry{Key: key, Val: value})
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
    fn runtime_declares_seq_and_map_types_and_helpers() {
        // SIR16 Sequences + Maps: the value model gains shared, mutable
        // `*Seq`/`*Map` arms and the lowering helpers for each IR node.
        assert!(RUNTIME.contains("type Seq struct"));
        assert!(RUNTIME.contains("Items []Value"));
        assert!(RUNTIME.contains("type Map struct"));
        assert!(RUNTIME.contains("type MapEntry struct"));
        for helper in &[
            "func _sir_seq_lit", "func _sir_seq_index", "func _sir_seq_len",
            "func _sir_seq_set", "func _sir_map_lit", "func _sir_map_get",
            "func _sir_map_set",
        ] {
            assert!(RUNTIME.contains(helper), "runtime missing `{}`", helper);
        }
    }

    #[test]
    fn runtime_declares_structural_value_eq() {
        // `=` and map-key lookup share one structural-equality function
        // that covers seqs and maps.
        assert!(RUNTIME.contains("func _sir_value_eq"));
        assert!(RUNTIME.contains("_sir_eq"));
    }

    #[test]
    fn runtime_seq_iter_handles_real_seq() {
        // ForEach reconciliation: `_sir_seq_iter` must snapshot a `*Seq`
        // (the new real sequence) as well as walk a cons-list.
        assert!(RUNTIME.contains("if s, ok := v.(*Seq); ok"));
    }

    #[test]
    fn runtime_formats_seq_and_map() {
        assert!(RUNTIME.contains("func _sir_format_seq"));
        assert!(RUNTIME.contains("func _sir_format_map"));
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
