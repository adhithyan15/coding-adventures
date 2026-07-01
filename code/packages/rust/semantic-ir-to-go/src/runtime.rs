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

// ── SIR19 default parameters: the MISSING sentinel ─────────────
//
// Emitted Go functions are *fixed-arity* over `Value` — Go has no native
// optional/default parameters.  To mimic call-time defaults we route an
// "argument not supplied" marker through the ordinary `Value` channel: a
// unique, package-level sentinel value.  A caller that omits a trailing
// defaulted argument PADS the call with `_sir_missing`; the callee's body
// prologue tests each defaulted param with `_sir_is_missing` and, when it
// finds the sentinel, evaluates that param's default expression in place
// (where earlier params are already bound — call-time, param-scope
// semantics).
//
// `_missingMarker` is a distinct, otherwise-empty struct type so the
// sentinel can never be confused with any user value: a program cannot
// construct a `*_missingMarker` itself (no IR node lowers to one), and
// pointer identity makes the `_sir_is_missing` check exact.  `_sir_missing`
// is the single shared instance boxed in a `Value`.
type _missingMarker struct{}

var _sir_missing Value = &_missingMarker{}

// True iff `v` is the missing-argument sentinel.  Compared by pointer
// identity (interface equality of the boxed `*_missingMarker`), so it is
// exact and total — no user value matches.
func _sir_is_missing(v Value) bool {
	_, ok := v.(*_missingMarker)
	return ok
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
	// Cycle safety: two *distinct* cyclic structures (e.g. `xs[0]=xs`
	// and `ys[0]=ys`, separate handles) would make a naive deep walk
	// recurse forever — the same-pointer fast path only catches a value
	// compared against *itself*.  We bound the walk co-inductively with a
	// `pending` set of handle-pairs currently being compared: re-
	// encountering a pair already in flight means we've closed a cycle in
	// lock-step, so we treat that pair as equal (the standard co-inductive
	// definition of bisimulation equality).  This terminates for *any*
	// pair of finite-handle graphs.  The public signature is unchanged: it
	// allocates a fresh `pending` set and delegates to the `_d` variant.
	return _sir_value_eq_d(a, b, make(map[[2]Value]bool))
}

func _sir_value_eq_d(a Value, b Value, pending map[[2]Value]bool) bool {
	// Defensive: the MISSING sentinel (SIR19 default params) never reaches
	// `=` in a well-formed program (a defaulted param is replaced by its
	// default before use).  Should one slip through, two sentinels are
	// equal and a sentinel equals nothing else — handled here before the
	// numeric/structural arms so it can never be mistaken for a value.
	if _sir_is_missing(a) || _sir_is_missing(b) {
		return _sir_is_missing(a) && _sir_is_missing(b)
	}
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
			return _sir_value_eq_d(ap.Car, bp.Car, pending) && _sir_value_eq_d(ap.Cdr, bp.Cdr, pending)
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
		// Already comparing this exact handle-pair higher up the stack ⇒
		// we've matched in lock-step around a cycle.  Assume equal; a
		// genuine difference is caught on a *non-cyclic* element
		// elsewhere.  Key the pair on the boxed `Value`s so the two
		// pointers compare by identity.
		key := [2]Value{a, b}
		if pending[key] {
			return true
		}
		if len(as.Items) != len(bs.Items) {
			return false
		}
		pending[key] = true
		result := true
		for i := range as.Items {
			if !_sir_value_eq_d(as.Items[i], bs.Items[i], pending) {
				result = false
				break
			}
		}
		delete(pending, key)
		return result
	}
	if am, ok := a.(*Map); ok {
		bm, ok := b.(*Map)
		if !ok {
			return false
		}
		if am == bm {
			return true
		}
		key := [2]Value{a, b}
		if pending[key] {
			return true
		}
		if len(am.Entries) != len(bm.Entries) {
			return false
		}
		pending[key] = true
		result := true
		for i := range am.Entries {
			if !_sir_value_eq_d(am.Entries[i].Key, bm.Entries[i].Key, pending) ||
				!_sir_value_eq_d(am.Entries[i].Val, bm.Entries[i].Val, pending) {
				result = false
				break
			}
		}
		delete(pending, key)
		return result
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

// ── format (cycle-safe) ────────────────────────────────────────
//
// `*Seq`/`*Map` are *shared, mutable* handles, so an emitted program
// can build a cyclic structure (`xs = []; xs[0] = xs`).  A naive
// structural walk would recurse forever and overflow the stack.  We
// guard the recursion with a `visited` set of the Seq/Map *pointers*
// currently on the active path: a handle is inserted on entry and
// removed on exit.
//
// Keying on the pointer is idiomatic in Go — a `*Seq`/`*Map` boxed in
// the `Value` (`interface{}`) compares by pointer identity, so it can be
// used directly as a `map[Value]bool` key.  Two `Value`s alias the same
// backing store iff they are the equal interface value (same dynamic
// type + same pointer).
//
// Removing on exit (rather than leaving it set for the whole walk) is
// deliberate — it means a value reached twice by two *sibling*
// (non-cyclic) paths still prints in full both times; only a handle that
// re-appears *within its own subtree* (a true cycle) is short-circuited
// to a placeholder (`[...]` for a seq, `{...}` for a map).
//
// `_sir_format(Value) string` keeps its public signature: it allocates a
// fresh visited set and delegates to the `_d` variant.
func _sir_format(v Value) string {
	return _sir_format_d(v, make(map[Value]bool))
}

func _sir_format_d(v Value, visited map[Value]bool) string {
	if v == nil {
		return "nil"
	}
	// Defensive: the MISSING sentinel (SIR19 default params) should never
	// reach a print path — a defaulted param is always replaced by its
	// default in the body prologue before any use.  Render it as a
	// distinctive marker rather than the bare `&{}` Go would otherwise
	// print, so a stray sentinel is obvious in output.
	if _sir_is_missing(v) {
		return "<missing>"
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
		return _sir_format_pair(p, visited)
	}
	if s, ok := v.(*Seq); ok {
		// Already on the active path ⇒ cycle.  Print a placeholder
		// instead of recursing forever.
		if visited[v] {
			return "[...]"
		}
		visited[v] = true
		out := _sir_format_seq(s, visited)
		delete(visited, v)
		return out
	}
	if m, ok := v.(*Map); ok {
		if visited[v] {
			return "{...}"
		}
		visited[v] = true
		out := _sir_format_map(m, visited)
		delete(visited, v)
		return out
	}
	if _, ok := v.(*Closure); ok {
		return "<closure>"
	}
	return fmt.Sprintf("%v", v)
}

// Sequences print like a bracketed list: `[1, 2, 3]`.
func _sir_format_seq(s *Seq, visited map[Value]bool) string {
	out := "["
	for i, item := range s.Items {
		if i > 0 {
			out += ", "
		}
		out += _sir_format_d(item, visited)
	}
	return out + "]"
}

// Maps print like a brace-wrapped entry list in insertion order:
// `{a: 1, b: 2}`.
func _sir_format_map(m *Map, visited map[Value]bool) string {
	out := "{"
	for i, e := range m.Entries {
		if i > 0 {
			out += ", "
		}
		out += _sir_format_d(e.Key, visited) + ": " + _sir_format_d(e.Val, visited)
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

// `Pair`s are immutable (no shared mutable handle), so a pair-chain can
// never form a cycle on its own.  It can, however, *contain* a cyclic
// seq/map in a `car`/`cdr`, so we still thread `visited` through to the
// element formatters.
func _sir_format_pair(p *Pair, visited map[Value]bool) string {
	out := "(" + _sir_format_d(p.Car, visited)
	rest := p.Cdr
	for {
		if next, ok := rest.(*Pair); ok {
			out += " " + _sir_format_d(next.Car, visited)
			rest = next.Cdr
			continue
		}
		if rest == nil {
			break
		}
		out += " . " + _sir_format_d(rest, visited)
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
//
// Cycle safety: unlike the Rust backend (whose `RefCell` would panic with
// "already mutably borrowed" if `value_eq` re-entered the map being
// mutated), Go has no aliasing-borrow check, so comparing a self-
// referential key here is sound on its own.  The remaining hazard — a
// cyclic key making `_sir_value_eq` recurse forever — is handled by that
// function's co-inductive `pending` guard, so a self-referential key
// (`d["self"] = d`) terminates.  No restructuring is needed here.
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

// ── Collection-method dispatch catalog (C5) ────────────────────
//
// `recv.meth(args…)` reaches this backend as
//   BuiltinCall("__method__", [recv, StrLit("meth"), …args])
// and is emitted as `_sir_call_method(recv, "meth", []Value{…})`.
// This is the Go analogue of the Python/TS `sir-runtime-oop`
// `call_method` catalog, ported for behavioural parity: the SAME
// method names and SAME semantics (Array/Hash/String/Numeric/Symbol),
// including block-passing (a trailing `*Closure` arg applied via
// `_sir_apply`) and `Symbol#to_proc` (`&:sym`, see `_sir_sym_to_proc`).
//
//   Ruby type  | Go representation
//   -----------|----------------------------------------------------
//   Array      | *Seq  (shared, mutable — push/<</pop mutate in place)
//   Hash       | *Map  (insertion-ordered assoc list)
//   String     | string
//   Integer    | int64
//   Float      | float64
//   Symbol     | *Symbol
//
// SECURITY (the C3 RCE lesson): dispatch is ONLY through the explicit
// name switches below — there is NO reflection on the raw method name,
// no dynamic Go method/field lookup.  The catalog switch IS the
// allowlist.  An unknown method name on a known receiver falls through
// to `_sir_method_unknown`, which panics with a clear, controlled
// message ("undefined method `bogus' for <type>") — a surfaced runtime
// error, never arbitrary behaviour.
//
// Return convention: every catalog helper returns `(Value, bool)` where
// the bool is `true` iff it recognised `name` for that receiver (a hit).
// A miss falls through to the next resolution tier, exactly mirroring the
// Python reference's `_MISS` sentinel.

// A block reaches a block-taking method as the LAST element of the args
// slice — a `*Closure` (an emitted `MakeClosure`, or a `_sir_sym_to_proc`
// for `&:sym`).  `_sir_split_block` peels a trailing closure off, so the
// leading positional args and the block are handed to the catalog
// separately (matching the Python reference's `arg_list[:-1], arg_list[-1]`).
func _sir_split_block(args []Value) ([]Value, *Closure) {
	if len(args) > 0 {
		if cl, ok := args[len(args)-1].(*Closure); ok {
			return args[:len(args)-1], cl
		}
	}
	return args, nil
}

// Ruby `to_s` display of a value used by `Array#join` (and elsewhere).
// The runtime's `_sir_format` already renders Ruby-ish forms EXCEPT that
// it prints booleans as Lisp `#t`/`#f` and `nil` as `nil`.  For the
// method catalog we want Ruby's surface (`true`/`false`, and `nil.to_s`
// == "" so it joins to nothing), so we special-case those here and defer
// to `_sir_format` for everything else.
func _sir_ruby_to_s(v Value) string {
	if v == nil {
		return ""
	}
	if b, ok := v.(bool); ok {
		if b {
			return "true"
		}
		return "false"
	}
	return _sir_format(v)
}

// ── Symbol#to_proc (&:sym) ─────────────────────────────────────
//
// Ruby's `&:sym` converts a Symbol to a block whose body calls the named
// method on its first argument, forwarding the rest.  So `xs.map(&:to_s)`
// is `xs.map { |x| x.to_s }` and `xs.inject(&:+)` is
// `inject { |a, x| a + x }`.  The frontend lowers `&:sym` to
// `block_pass(SymLit("sym"))`; the backend emits `_sir_sym_to_proc(
// _sir_intern("sym"))`, yielding a `*Closure` the block-taking catalog
// drives through `_sir_apply` exactly like a `{ }` block.  Applying it to
// `[recv, rest…]` re-enters `_sir_call_method(recv, "sym", rest…)`, so an
// out-of-catalog method surfaces the same controlled failure as a direct
// call.
func _sir_sym_to_proc(sym Value) Value {
	name := ""
	if s, ok := sym.(*Symbol); ok {
		name = s.Name
	} else if s, ok := sym.(string); ok {
		name = s
	}
	return &Closure{Fn: func(args []Value) Value {
		if len(args) == 0 {
			return nil
		}
		return _sir_call_method(args[0], name, args[1:])
	}}
}

// Public dispatch entry point emitted for every `__method__` call.
func _sir_call_method(recv Value, name string, args []Value) Value {
	// Type-specific catalogs first (a String is neither Seq nor Map).
	// `bool` is checked before the numeric arm so `true`/`false` never
	// enter the numeric catalog (they resolve only universal methods).
	switch r := recv.(type) {
	case string:
		if v, ok := _sir_string_method(r, name, args); ok {
			return v
		}
	case *Symbol:
		if v, ok := _sir_symbol_method(r, name, args); ok {
			return v
		}
	case bool:
		// bool has no dedicated catalog here beyond the universal
		// methods handled below; fall through.
	case int64, int, float64:
		if v, ok := _sir_numeric_method(recv, name, args); ok {
			return v
		}
	case *Seq:
		if v, ok := _sir_array_method(r, name, args); ok {
			return v
		}
	case *Map:
		if v, ok := _sir_hash_method(r, name, args); ok {
			return v
		}
	}
	// Universal Object methods available on every receiver.
	if v, ok := _sir_object_method(recv, name, args); ok {
		return v
	}
	// No catalog entry matched → a controlled, clearly-messaged failure.
	// NEVER a reflective fallthrough (the C3 allowlist discipline).
	return _sir_method_unknown(recv, name)
}

// Clean, controlled failure for an unknown method — panics with a Ruby
// `NoMethodError`-shaped message.  A `go run` surfaces it as a non-zero
// exit plus this message, exactly like `car` on a non-pair — NOT a silent
// nil or arbitrary behaviour.
func _sir_method_unknown(recv Value, name string) Value {
	panic("undefined method `" + name + "' for " + _sir_ruby_class_name(recv))
}

// Conventional Ruby class name of a value (for error messages / `class`).
func _sir_ruby_class_name(v Value) string {
	if v == nil {
		return "NilClass"
	}
	switch t := v.(type) {
	case bool:
		if t {
			return "TrueClass"
		}
		return "FalseClass"
	case int64, int:
		return "Integer"
	case float64:
		return "Float"
	case string:
		return "String"
	case *Symbol:
		return "Symbol"
	case *Seq:
		return "Array"
	case *Map:
		return "Hash"
	}
	return "Object"
}

// ── Universal Object methods ───────────────────────────────────
func _sir_object_method(recv Value, name string, args []Value) (Value, bool) {
	switch name {
	case "nil?":
		return recv == nil, true
	case "==":
		return _sir_value_eq(recv, args[0]), true
	case "!=":
		return !_sir_value_eq(recv, args[0]), true
	case "class":
		return _sir_ruby_class_name(recv), true
	case "to_s":
		return _sir_ruby_to_s(recv), true
	case "itself":
		return recv, true
	}
	return nil, false
}

// ── Array (*Seq) catalog ───────────────────────────────────────
func _sir_array_method(recv *Seq, name string, args []Value) (Value, bool) {
	// Block-taking methods are dispatched only when a trailing *Closure
	// is present; peel it off the positional args first.
	pos, block := _sir_split_block(args)
	if block != nil {
		if v, ok := _sir_array_block_method(recv, name, pos, block); ok {
			return v, true
		}
	}
	switch name {
	case "length", "size", "count":
		if name == "count" && len(args) > 0 {
			n := int64(0)
			for _, x := range recv.Items {
				if _sir_value_eq(x, args[0]) {
					n++
				}
			}
			return n, true
		}
		return int64(len(recv.Items)), true
	case "first":
		if len(recv.Items) == 0 {
			return nil, true
		}
		return recv.Items[0], true
	case "last":
		if len(recv.Items) == 0 {
			return nil, true
		}
		return recv.Items[len(recv.Items)-1], true
	case "empty?":
		return len(recv.Items) == 0, true
	case "include?":
		for _, x := range recv.Items {
			if _sir_value_eq(x, args[0]) {
				return true, true
			}
		}
		return false, true
	case "index":
		for i, x := range recv.Items {
			if _sir_value_eq(x, args[0]) {
				return int64(i), true
			}
		}
		return nil, true
	case "push", "append":
		// Mutate the shared handle in place (Ruby `push`/`<<` mutate);
		// return the receiver so `xs.push(4)` chains.
		recv.Items = append(recv.Items, args...)
		return recv, true
	case "<<":
		recv.Items = append(recv.Items, args[0])
		return recv, true
	case "pop":
		if len(recv.Items) == 0 {
			return nil, true
		}
		last := recv.Items[len(recv.Items)-1]
		recv.Items = recv.Items[:len(recv.Items)-1]
		return last, true
	case "shift":
		if len(recv.Items) == 0 {
			return nil, true
		}
		first := recv.Items[0]
		recv.Items = recv.Items[1:]
		return first, true
	case "reverse":
		out := make([]Value, len(recv.Items))
		for i, x := range recv.Items {
			out[len(recv.Items)-1-i] = x
		}
		return &Seq{Items: out}, true
	case "sort":
		out := make([]Value, len(recv.Items))
		copy(out, recv.Items)
		sort.SliceStable(out, func(i, j int) bool {
			return _sir_value_lt(out[i], out[j])
		})
		return &Seq{Items: out}, true
	case "join":
		sep := ""
		if len(args) > 0 {
			if s, ok := args[0].(string); ok {
				sep = s
			}
		}
		parts := make([]string, len(recv.Items))
		for i, x := range recv.Items {
			parts[i] = _sir_ruby_to_s(x)
		}
		return strings.Join(parts, sep), true
	case "to_a":
		return recv, true
	}
	return nil, false
}

// Block-taking Array/Enumerable methods.  `block` is applied via
// `_sir_apply` (proc-lenient); predicate results route through
// `_sir_truthy` (only false/nil are falsy).
func _sir_array_block_method(recv *Seq, name string, args []Value, block *Closure) (Value, bool) {
	switch name {
	case "each":
		for _, x := range recv.Items {
			_sir_apply(block, []Value{x})
		}
		return recv, true
	case "map", "collect":
		out := make([]Value, len(recv.Items))
		for i, x := range recv.Items {
			out[i] = _sir_apply(block, []Value{x})
		}
		return &Seq{Items: out}, true
	case "select", "filter":
		out := []Value{}
		for _, x := range recv.Items {
			if _sir_truthy(_sir_apply(block, []Value{x})) {
				out = append(out, x)
			}
		}
		return &Seq{Items: out}, true
	case "reject":
		out := []Value{}
		for _, x := range recv.Items {
			if !_sir_truthy(_sir_apply(block, []Value{x})) {
				out = append(out, x)
			}
		}
		return &Seq{Items: out}, true
	case "reduce", "inject":
		var acc Value
		var rest []Value
		if len(args) > 0 {
			acc = args[0]
			rest = recv.Items
		} else if len(recv.Items) > 0 {
			acc = recv.Items[0]
			rest = recv.Items[1:]
		} else {
			return nil, true
		}
		for _, x := range rest {
			acc = _sir_apply(block, []Value{acc, x})
		}
		return acc, true
	case "find", "detect":
		for _, x := range recv.Items {
			if _sir_truthy(_sir_apply(block, []Value{x})) {
				return x, true
			}
		}
		return nil, true
	case "any?":
		for _, x := range recv.Items {
			if _sir_truthy(_sir_apply(block, []Value{x})) {
				return true, true
			}
		}
		return false, true
	case "all?":
		for _, x := range recv.Items {
			if !_sir_truthy(_sir_apply(block, []Value{x})) {
				return false, true
			}
		}
		return true, true
	case "none?":
		for _, x := range recv.Items {
			if _sir_truthy(_sir_apply(block, []Value{x})) {
				return false, true
			}
		}
		return true, true
	}
	return nil, false
}

// Ordering used by `Array#sort`.  Numbers compare numerically, strings
// lexicographically, symbols by name; a mixed/uncomparable pair keeps a
// stable order (returns false) rather than panicking — the never-raise
// floor for the OO surface.
func _sir_value_lt(a Value, b Value) bool {
	if _sir_is_number_val(a) && _sir_is_number_val(b) {
		return _sir_as_float(a) < _sir_as_float(b)
	}
	if as, ok := a.(string); ok {
		if bs, ok := b.(string); ok {
			return as < bs
		}
	}
	if as, ok := a.(*Symbol); ok {
		if bs, ok := b.(*Symbol); ok {
			return as.Name < bs.Name
		}
	}
	return false
}

// ── Hash (*Map) catalog ────────────────────────────────────────
func _sir_hash_method(recv *Map, name string, args []Value) (Value, bool) {
	pos, block := _sir_split_block(args)
	if block != nil {
		if v, ok := _sir_hash_block_method(recv, name, pos, block); ok {
			return v, true
		}
	}
	switch name {
	case "keys":
		out := make([]Value, len(recv.Entries))
		for i, e := range recv.Entries {
			out[i] = e.Key
		}
		return &Seq{Items: out}, true
	case "values":
		out := make([]Value, len(recv.Entries))
		for i, e := range recv.Entries {
			out[i] = e.Val
		}
		return &Seq{Items: out}, true
	case "has_key?", "key?", "include?", "member?":
		for _, e := range recv.Entries {
			if _sir_value_eq(e.Key, args[0]) {
				return true, true
			}
		}
		return false, true
	case "has_value?", "value?":
		for _, e := range recv.Entries {
			if _sir_value_eq(e.Val, args[0]) {
				return true, true
			}
		}
		return false, true
	case "size", "length":
		return int64(len(recv.Entries)), true
	case "empty?":
		return len(recv.Entries) == 0, true
	}
	return nil, false
}

func _sir_hash_block_method(recv *Map, name string, args []Value, block *Closure) (Value, bool) {
	switch name {
	case "each", "each_pair":
		for _, e := range recv.Entries {
			_sir_apply(block, []Value{e.Key, e.Val})
		}
		return recv, true
	case "map":
		out := make([]Value, len(recv.Entries))
		for i, e := range recv.Entries {
			out[i] = _sir_apply(block, []Value{e.Key, e.Val})
		}
		return &Seq{Items: out}, true
	case "select", "filter":
		m := &Map{Entries: []MapEntry{}}
		for _, e := range recv.Entries {
			if _sir_truthy(_sir_apply(block, []Value{e.Key, e.Val})) {
				m.Entries = append(m.Entries, MapEntry{Key: e.Key, Val: e.Val})
			}
		}
		return m, true
	case "reject":
		m := &Map{Entries: []MapEntry{}}
		for _, e := range recv.Entries {
			if !_sir_truthy(_sir_apply(block, []Value{e.Key, e.Val})) {
				m.Entries = append(m.Entries, MapEntry{Key: e.Key, Val: e.Val})
			}
		}
		return m, true
	}
	return nil, false
}

// ── String catalog ─────────────────────────────────────────────
//
// A Ruby String is an immutable Go `string`, so every method returns a
// fresh value (nothing mutates in place).
func _sir_string_method(recv string, name string, args []Value) (Value, bool) {
	switch name {
	case "length", "size":
		return int64(len([]rune(recv))), true
	case "upcase":
		return strings.ToUpper(recv), true
	case "downcase":
		return strings.ToLower(recv), true
	case "reverse":
		r := []rune(recv)
		for i, j := 0, len(r)-1; i < j; i, j = i+1, j-1 {
			r[i], r[j] = r[j], r[i]
		}
		return string(r), true
	case "strip":
		return strings.TrimSpace(recv), true
	case "lstrip":
		return strings.TrimLeft(recv, " \t\r\n\f\v"), true
	case "rstrip":
		return strings.TrimRight(recv, " \t\r\n\f\v"), true
	case "empty?":
		return len(recv) == 0, true
	case "include?":
		if s, ok := args[0].(string); ok {
			return strings.Contains(recv, s), true
		}
		return false, true
	case "start_with?":
		if s, ok := args[0].(string); ok {
			return strings.HasPrefix(recv, s), true
		}
		return false, true
	case "end_with?":
		if s, ok := args[0].(string); ok {
			return strings.HasSuffix(recv, s), true
		}
		return false, true
	case "split":
		// No separator ⇒ split on runs of whitespace (Ruby's awk-style
		// default); with a separator ⇒ split on that literal substring.
		var parts []string
		if len(args) == 0 {
			parts = strings.Fields(recv)
		} else if s, ok := args[0].(string); ok {
			parts = strings.Split(recv, s)
		} else {
			parts = strings.Fields(recv)
		}
		out := make([]Value, len(parts))
		for i, p := range parts {
			out[i] = p
		}
		return &Seq{Items: out}, true
	case "chars":
		r := []rune(recv)
		out := make([]Value, len(r))
		for i, c := range r {
			out[i] = string(c)
		}
		return &Seq{Items: out}, true
	case "to_i":
		return _sir_str_to_i(recv), true
	case "to_f":
		return _sir_str_to_f(recv), true
	case "to_sym":
		return _sir_intern(recv), true
	}
	return nil, false
}

// Ruby `String#to_i`: parse the longest leading (optionally-signed)
// integer run after trimming whitespace; yield 0 when nothing leads
// (Ruby never raises here, unlike Go's `strconv.Atoi`).
func _sir_str_to_i(s string) Value {
	s = strings.TrimSpace(s)
	i := 0
	if i < len(s) && (s[i] == '+' || s[i] == '-') {
		i++
	}
	j := i
	for j < len(s) && s[j] >= '0' && s[j] <= '9' {
		j++
	}
	if j == i {
		return int64(0)
	}
	n, err := strconv.ParseInt(s[:j], 10, 64)
	if err != nil {
		return int64(0)
	}
	return n
}

// Ruby `String#to_f`: leading float, else 0.0.  We grow the longest
// prefix that still parses as a float via `strconv.ParseFloat`.
func _sir_str_to_f(s string) Value {
	s = strings.TrimSpace(s)
	best := 0.0
	for j := 1; j <= len(s); j++ {
		if f, err := strconv.ParseFloat(s[:j], 64); err == nil {
			best = f
		}
	}
	return best
}

// ── Numeric (Integer/Float) catalog ────────────────────────────
func _sir_numeric_method(recv Value, name string, args []Value) (Value, bool) {
	// Block-taking `times` is dispatched when a trailing *Closure is present.
	_, block := _sir_split_block(args)
	if block != nil && name == "times" {
		n := _sir_as_int(recv)
		for i := int64(0); i < n; i++ {
			_sir_apply(block, []Value{i})
		}
		return recv, true
	}
	isInt := false
	switch recv.(type) {
	case int64, int:
		isInt = true
	}
	switch name {
	case "abs":
		if isInt {
			n := _sir_as_int(recv)
			if n < 0 {
				return -n, true
			}
			return n, true
		}
		return math.Abs(_sir_as_float(recv)), true
	case "to_i":
		return _sir_as_int_trunc(recv), true
	case "to_f":
		return _sir_as_float(recv), true
	case "even?":
		return _sir_as_int_trunc(recv)%2 == 0, true
	case "odd?":
		return _sir_as_int_trunc(recv)%2 != 0, true
	case "zero?":
		return _sir_as_float(recv) == 0, true
	case "positive?":
		return _sir_as_float(recv) > 0, true
	case "negative?":
		return _sir_as_float(recv) < 0, true
	case "succ", "next":
		if isInt {
			return _sir_as_int(recv) + 1, true
		}
		return _sir_as_float(recv) + 1, true
	case "pred":
		if isInt {
			return _sir_as_int(recv) - 1, true
		}
		return _sir_as_float(recv) - 1, true
	}
	return nil, false
}

// `to_i`-style truncation that also accepts a float receiver (Ruby's
// `3.7.to_i == 3`, `even?`/`odd?` truncate first).  A non-finite float
// degrades to 0 rather than panicking (never-raise floor).
func _sir_as_int_trunc(v Value) int64 {
	switch n := v.(type) {
	case int64:
		return n
	case int:
		return int64(n)
	case float64:
		if math.IsNaN(n) || math.IsInf(n, 0) {
			return 0
		}
		return int64(math.Trunc(n))
	}
	return 0
}

// ── Symbol catalog ─────────────────────────────────────────────
func _sir_symbol_method(recv *Symbol, name string, args []Value) (Value, bool) {
	switch name {
	case "to_s":
		return recv.Name, true
	case "to_sym":
		return recv, true
	case "length", "size":
		return int64(len([]rune(recv.Name))), true
	case "upcase":
		return _sir_intern(strings.ToUpper(recv.Name)), true
	case "downcase":
		return _sir_intern(strings.ToLower(recv.Name)), true
	case "empty?":
		return len(recv.Name) == 0, true
	}
	return nil, false
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
    fn runtime_format_is_cycle_safe() {
        // The public `_sir_format(Value) string` delegates to a
        // visited-set variant that threads a `map[Value]bool` of the
        // Seq/Map pointers currently on the active path, emitting a
        // placeholder on re-entry (a true cycle) so a cyclic value
        // terminates instead of overflowing the stack.
        assert!(RUNTIME.contains("func _sir_format_d(v Value, visited map[Value]bool) string"));
        assert!(RUNTIME.contains("return _sir_format_d(v, make(map[Value]bool))"));
        assert!(RUNTIME.contains("\"[...]\""));
        assert!(RUNTIME.contains("\"{...}\""));
    }

    #[test]
    fn runtime_value_eq_is_cycle_safe() {
        // `_sir_value_eq` keeps the same-pointer fast path and adds a
        // co-inductive `pending` set of handle-pairs (`map[[2]Value]bool`)
        // currently being compared, so two *distinct* cyclic structures
        // terminate (lock-step cycle ⇒ equal).
        assert!(RUNTIME.contains("func _sir_value_eq_d(a Value, b Value, pending map[[2]Value]bool) bool"));
        assert!(RUNTIME.contains("return _sir_value_eq_d(a, b, make(map[[2]Value]bool))"));
        assert!(RUNTIME.contains("key := [2]Value{a, b}"));
    }

    #[test]
    fn runtime_declares_missing_sentinel_and_helper() {
        // SIR19 default params: the runtime must carry a unique MISSING
        // sentinel (a distinct `*_missingMarker` boxed in a `Value`) and an
        // exact `_sir_is_missing` predicate.
        assert!(RUNTIME.contains("type _missingMarker struct{}"));
        assert!(RUNTIME.contains("var _sir_missing Value = &_missingMarker{}"));
        assert!(RUNTIME.contains("func _sir_is_missing(v Value) bool"));
    }

    #[test]
    fn runtime_handles_missing_in_format_and_eq() {
        // The sentinel never normally prints or compares, but both paths
        // guard it defensively so a stray sentinel can't masquerade as a
        // user value.
        assert!(RUNTIME.contains("\"<missing>\""));
        assert!(RUNTIME.contains("if _sir_is_missing(a) || _sir_is_missing(b)"));
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
