//! Inlined Go runtime helpers — pasted verbatim into every artifact.
//!
//! Imports of `fmt`, `math`, and `strconv` are required by the runtime;
//! the emitter always emits them in the file header.  They are always
//! used (the runtime block as a whole references all three — `math` via
//! the SIR16 float `NaN`/`Inf` checks in `_sir_format_float`), so the Go
//! "unused import" rule is satisfied for every generated file.

pub const RUNTIME: &str = r##"// ── inlined SIR runtime ────────────────────────────────────────

// Source-language display convention (SIR display-convention spec).  The
// emitter substitutes `__SIR_DISPLAY_RUBY__` with `true` when the module's
// `source_language` is Ruby, else `false` (the default Twig/Lisp form).  The
// display path (`_sir_format`) reads this to render a boolean as Ruby
// `true`/`false` rather than the Lisp `#t`/`#f`.  A compile-time `const` → the
// Go compiler folds the branch away; existing Twig output is unchanged.
const _sir_display_ruby = __SIR_DISPLAY_RUBY__

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

// ── polymorphic `+` (sir-polymorphic-operators PO4) ────────────
//
// Ruby overloads `+` by receiver type; all cases lower to `_sir_plus`,
// so the runtime dispatches on the FIRST operand's tag (a Go type switch
// — NEVER reflection, per the [[dynamic-dispatch-rce]] discipline):
//
//   | args[0] tag | behaviour                                        |
//   |-------------|--------------------------------------------------|
//   | string      | concatenate ALL operands as strings → string     |
//   | *Seq        | concatenate element slices → NEW *Seq (no alias) |
//   | otherwise   | numeric fold (int/float promotion), unchanged    |
//
// Ruby `+` is binary, but the SIR builtin is variadic; the string/array
// arms fold left-associatively over ≥2 operands, preserving the existing
// variadic contract of the numeric path.
func _sir_plus(args []Value) Value {
	if len(args) > 0 {
		switch first := args[0].(type) {
		case string:
			// String concat.  Every operand must itself be a string —
			// Ruby raises TypeError on `"a" + 1` (deferred to the
			// typed-runtime-errors cascade); here `_sir_as_string`
			// gives a controlled panic rather than silent garbage.
			var out string
			for _, a := range args {
				out += _sir_as_string(a)
			}
			return out
		case *Seq:
			// Array concat: build a FRESH backing slice so the result
			// never aliases any input's backing array (Ruby `+` returns
			// a new array; only `concat`/`<<` mutate in place).
			out := make([]Value, 0, len(first.Items))
			out = append(out, first.Items...)
			for _, a := range args[1:] {
				s, ok := a.(*Seq)
				if !ok {
					panic("no implicit conversion of " + _sir_ruby_class_name(a) + " into Array")
				}
				out = append(out, s.Items...)
			}
			return &Seq{Items: out}
		}
	}
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

// Coerce an operand to a Go string for the string `+` arm.  A genuine
// string passes through; anything else is a controlled panic (Ruby would
// raise TypeError — the typed-runtime-errors cascade will refine this).
func _sir_as_string(v Value) string {
	if s, ok := v.(string); ok {
		return s
	}
	panic("no implicit conversion of " + _sir_ruby_class_name(v) + " into String")
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

// ── polymorphic `*` (sir-polymorphic-operators PO4) ────────────
//
// Ruby `*` is binary and overloaded by the RECEIVER (first operand):
//
//   | args[0]  | args[1] | behaviour                                    |
//   |----------|---------|----------------------------------------------|
//   | string   | Integer | repeat the string N times ("ab"*3 → ababab)  |
//   | string   | int ≤ 0 | "" (Ruby raises on negative, but "" is the   |
//   |          |         |   never-raise floor here)                    |
//   | *Seq     | Integer | repeat the element list N times ([0]*3)      |
//   | *Seq     | string  | join elements with the separator ([1,2]*", ")|
//   | otherwise| —       | numeric fold (int/float promotion), unchanged|
//
// Dispatch is on the runtime tag via a Go type switch — never reflection.
// The string/array arms handle the common BINARY case (Ruby `*` is
// binary); anything else falls through to the variadic numeric fold, so
// existing numeric semantics are preserved exactly.
func _sir_times(args []Value) Value {
	if len(args) == 2 {
		switch recv := args[0].(type) {
		case string:
			// String × Integer → repeat.  A non-positive count (or an
			// empty receiver) yields the empty string.  Guard the
			// product len(recv)*n against host-int overflow: Ruby raises
			// `ArgumentError: argument too big` for an oversized repeat,
			// so we panic with the same controlled message rather than
			// let strings.Repeat overflow `int` (opaque panic) or attempt
			// a multi-gigabyte allocation and OOM the process.
			n := _sir_as_int(args[1])
			if n <= 0 || len(recv) == 0 {
				return ""
			}
			maxInt := int64(^uint(0) >> 1)
			if n > maxInt/int64(len(recv)) {
				panic("argument too big")
			}
			return strings.Repeat(recv, int(n))
		case *Seq:
			// Seq × string → join with separator, using the SAME
			// value-display helper the runtime uses for `puts`
			// (`_sir_format`), so an element renders identically whether
			// printed or joined.
			if sep, ok := args[1].(string); ok {
				parts := make([]string, len(recv.Items))
				for i, x := range recv.Items {
					parts[i] = _sir_format(x)
				}
				return strings.Join(parts, sep)
			}
			// Seq × Integer → repeat the element list into a FRESH
			// backing slice (no aliasing of the input).  A non-positive
			// count (or an empty receiver) yields an empty array; the
			// empty-receiver short-circuit also avoids spinning the
			// append loop for a huge count.  Guard len(Items)*n against
			// host-int overflow (Ruby raises `ArgumentError: argument
			// too big`) so `make` never receives a wrapped/negative cap.
			n := _sir_as_int(args[1])
			if n <= 0 || len(recv.Items) == 0 {
				return &Seq{Items: []Value{}}
			}
			maxInt := int64(^uint(0) >> 1)
			if n > maxInt/int64(len(recv.Items)) {
				panic("argument too big")
			}
			out := make([]Value, 0, len(recv.Items)*int(n))
			for i := int64(0); i < n; i++ {
				out = append(out, recv.Items...)
			}
			return &Seq{Items: out}
		}
	}
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
		// Ruby raises `ZeroDivisionError` for division by zero on BOTH the
		// integer AND float paths (`1/0` and `1.0/0` alike — see the
		// sir-typed-runtime-errors spec, which is load-bearing here).  We
		// therefore reject a zero divisor before the promoted float divide,
		// rather than letting IEEE-754 yield `+Inf`.  The typed `SirError`
		// (raised via the existing `_sir_new_error` entry point + `panic`,
		// exactly as an explicit `raise ZeroDivisionError` would) is what a
		// translated `rescue ZeroDivisionError` matches.
		acc := _sir_as_float(args[0])
		for _, a := range args[1:] {
			d := _sir_as_float(a)
			if d == 0 {
				panic(_sir_new_error("ZeroDivisionError", Value("divided by 0")))
			}
			acc /= d
		}
		return acc
	}
	acc := _sir_as_int(args[0])
	for _, a := range args[1:] {
		d := _sir_as_int(a)
		if d == 0 {
			panic(_sir_new_error("ZeroDivisionError", Value("divided by 0")))
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

// _sir_case_eq implements Ruby case-equality (`pattern === value`) — the test a
// `when` (or `in`) arm runs.  Unlike `==`, Ruby keys `===` to the PATTERN's
// type:
//
//	| pattern kind | semantics                          |
//	|--------------|------------------------------------|
//	| Range        | membership (`value` falls in range)|
//	| Regexp       | the regex matches `value`          |
//	| anything else| value equality (`==`)              |
//
// A `when SomeClass` is lowered to `value.is_a?(SomeClass)` at the FRONTEND
// (`__method__` dispatch), so a class pattern never reaches here.  This
// backend's Value model has no Range or Regexp variant yet, so the only
// patterns that reach `case_eq` are ordinary values and the operation is
// exactly structural equality — matching the Python reference in
// `sir-runtime-oop`.  When Range/Regexp values are added, extend this with the
// membership/match arms (dispatching on the pattern, args[0]).
func _sir_case_eq(args []Value) Value {
	if len(args) < 2 {
		return false
	}
	return _sir_value_eq(args[0], args[1])
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

// ── puts (Ruby semantics) ──────────────────────────────────────
//
// Ruby's `puts` is THE common output method and is deceptively subtle:
//
//   - `puts`            → one newline.
//   - `puts x`          → `x.to_s` then a newline, UNLESS `x.to_s` already
//                         ends in "\n" (then no second newline is added):
//                         `puts "x\n"` prints `x\n`, not `x\n\n`.
//   - `puts a, b`       → each argument on its own line, in order.
//   - `puts nil`        → a blank line (`nil.to_s` is "", then the newline).
//   - `puts []`         → a single newline (an argument that flattens to
//                         nothing still prints a blank line).
//   - `puts [1,[2,3]]`  → each ELEMENT on its own line, arrays flattened
//                         recursively: `1\n2\n3\n`.
//
// `puts` is variadic, so it takes the whole `[]Value` (unlike the fixed-arity
// `_sir_print`).  We write raw bytes with `fmt.Print`/`os.Stdout` rather than
// `fmt.Println` so the trailing-newline-suppression rule can be honoured.
func _sir_puts(args []Value) Value {
	if len(args) == 0 {
		// No arguments: exactly one newline.
		fmt.Print("\n")
		return nil
	}
	// A `*Seq` is a shared, mutable handle, so a program can build a
	// *cyclic* array (`a = []; a << a`).  The element-per-line flatten below
	// recurses through nested arrays, so — like `_sir_format` — it MUST be
	// cycle-guarded or a self-referential array overflows the Go stack (a
	// DoS: CWE-674, uncontrolled recursion).  We thread a `visited` set of
	// the `*Seq` pointers on the active flatten path; the top-level args each
	// share one set (a handle removed on exit still prints in full via a
	// sibling path — only a true self-cycle is short-circuited).
	visited := make(map[Value]bool)
	for _, a := range args {
		// `puts []` (empty array arg) still writes one blank line — Ruby
		// prints a line when an argument flattens to nothing.  A recursive
		// flatten of an empty seq writes nothing, so detect it here.
		if s, ok := a.(*Seq); ok && len(s.Items) == 0 {
			fmt.Print("\n")
			continue
		}
		_sir_puts_one(a, visited)
	}
	return nil
}

// Emit a single `puts` argument.  Arrays recurse (element-per-line, nested
// arrays flattened); everything else renders via `_sir_format` then a
// newline — suppressed when the text already ends in one.  `nil` is a blank
// line (`_sir_format(nil)` is "nil" for `print`, but `puts nil` is a blank
// line, so nil is special-cased).
//
// Cycle safety: `visited` holds the `*Seq` pointers currently on the active
// flatten path.  A seq ALREADY on the path is a cycle (`a = []; a << a`):
// rather than recurse forever we write Ruby's `[...]` placeholder then a
// newline, matching real Ruby (`puts a` on a self-referential array prints
// `[...]` and terminates).  (We emit the literal placeholder rather than
// `_sir_format(v)`: that formatter starts a fresh visited set, so it would
// render the *containing* level too — `[[...]]` for `a = [a]` — whereas Ruby
// prints a bare `[...]`.)  A seq reached twice by *sibling* (non-cyclic) paths
// is fully flattened both times, because each is removed from `visited` on
// exit — only a handle re-appearing *within its own subtree* is short-
// circuited.  Non-cyclic output is unchanged (`puts [1,[2,3]]` still prints
// `1\n2\n3\n`).
func _sir_puts_one(v Value, visited map[Value]bool) {
	if s, ok := v.(*Seq); ok {
		if visited[v] {
			fmt.Print("[...]\n")
			return
		}
		visited[v] = true
		for _, item := range s.Items {
			_sir_puts_one(item, visited)
		}
		delete(visited, v)
		return
	}
	if v == nil {
		fmt.Print("\n")
		return
	}
	text := _sir_format(v)
	if strings.HasSuffix(text, "\n") {
		fmt.Print(text)
	} else {
		fmt.Print(text + "\n")
	}
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
			if _sir_display_ruby {
				return "true"
			}
			return "#t"
		}
		if _sir_display_ruby {
			return "false"
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
	// A SIR exception prints as its message (Ruby's `exception.message`):
	// the raised message when present, else the class name.  This lets a
	// `rescue => e` do `print(e)` and see something meaningful rather than
	// Go's default `&{...}` struct rendering.
	if se, ok := v.(*SirError); ok {
		if se.Msg == nil {
			return se.Class
		}
		return _sir_format_d(se.Msg, visited)
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
	// Ruby `arr[i]` (the `[]` index op, NOT `arr.fetch(i)`): a negative index
	// counts from the end (`arr[-1]` is the last element); an index that still
	// falls outside `0 .. len-1` returns **nil** — it does NOT raise.  (Only
	// `fetch` raises IndexError.)  This matches the sir spec and the Python/
	// JS/TS/Rust backends; previously Go panicked on any OOB, diverging.
	n := int64(len(s.Items))
	if i < 0 {
		i += n
	}
	if i < 0 || i >= n {
		return nil
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
	case "case_eq":
		return _sir_case_eq(args)
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
	case "puts":
		return _sir_puts(args)
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
	// ── O4 user-object path ────────────────────────────────────────
	//
	// When `recv` is a user `*SirInstance`, a method registered via
	// `_sir_def_method` (walking the class's ancestry through the shared
	// `_sir_ancestry` table) is dispatched FIRST: push the receiver as the
	// current self, apply the stored closure, pop via `defer` (panic-safe).
	// Resolution is an EXPLICIT `(class, method)` map lookup — never
	// reflection — so a method named `constructor`/`__proto__` simply misses.
	//
	// Only if NO user method resolves does dispatch FALL THROUGH to the
	// universal Object methods below (so `obj.class` / `obj.nil?` still work
	// on an instance).  A `*SirInstance` is neither Seq/Map/String/etc., so
	// it skips the collection/primitive catalogs entirely and is UNCHANGED
	// for those receiver types.
	if inst, ok := recv.(*SirInstance); ok {
		if fn, found := _sir_resolve_instance_method(inst.Class, name); found {
			_sir_push_self(inst)
			defer _sir_pop_self()
			return _sir_apply(fn, args)
		}
		// M6: `send`/`tap`/`then` apply to a user instance too, AFTER its own
		// methods (so a user-defined `send`/`tap` override wins, resolution
		// order #2).  These recurse / apply a block, so route them here before
		// the universal Object fallback.
		if v, ok := _sir_meta_method(recv, name, args); ok {
			return v
		}
		// Universal Object methods (class/nil?/==/…) still apply to an
		// instance; anything else is the controlled NoMethodError floor.
		if v, ok := _sir_object_method(recv, name, args); ok {
			return v
		}
		return _sir_method_unknown(recv, name)
	}
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
		// M6: `TrueClass`/`FalseClass` logical operators (`&`/`|`/`^`)
		// resolve BEFORE the universal Object table so `true & false`
		// runs rather than bottoming out at NoMethodError.
		if v, ok := _sir_bool_method(r, name, args); ok {
			return v
		}
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
	// M6 universal metaprogramming (`send`/`tap`/`then`/`yield_self`):
	// dispatched AFTER the type-specific catalogs (so a catalog method of the
	// same name wins) and BEFORE the universal Object fallback.  `send`
	// re-enters dispatch with a dynamic name; `tap`/`then` apply a trailing
	// block.  See `_sir_meta_method`.
	if v, ok := _sir_meta_method(recv, name, args); ok {
		return v
	}
	// Universal Object methods available on every receiver.
	if v, ok := _sir_object_method(recv, name, args); ok {
		return v
	}
	// No catalog entry matched → a controlled, clearly-messaged failure.
	// NEVER a reflective fallthrough (the C3 allowlist discipline).
	return _sir_method_unknown(recv, name)
}

// Clean, controlled failure for an unknown method — raises a TYPED Ruby
// `NoMethodError` (via the existing `_sir_new_error` entry point + `panic`,
// exactly as an explicit `raise NoMethodError, msg` would) so a translated
// `rescue NoMethodError` catches it.  The message mirrors Ruby's shape,
// `undefined method 'x' for <class>`.  A `go run` with no surrounding
// rescue surfaces it as a non-zero exit plus this message — NOT a silent
// nil or arbitrary behaviour.
func _sir_method_unknown(recv Value, name string) Value {
	panic(_sir_new_error("NoMethodError",
		Value("undefined method '"+name+"' for "+_sir_ruby_class_name(recv))))
}

// Conventional Ruby class name of a value (for error messages / `class`).
func _sir_ruby_class_name(v Value) string {
	if v == nil {
		return "NilClass"
	}
	// A user instance reports its own class tag (so `obj.class` and a
	// NoMethodError message name the real class, e.g. `Dog`).
	if inst, ok := v.(*SirInstance); ok {
		return inst.Class
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
	case "inspect":
		// Ruby `inspect` renders a debuggable form: strings are quoted,
		// `nil`/`true`/`false` are their keywords.  `_sir_format` already
		// produces the debug surface for collections/strings; the only
		// divergence from `_sir_format` is booleans (`#t`/`#f` → true/false)
		// and nil (already `nil`), so route through it after fixing bools.
		if b, ok := recv.(bool); ok {
			if b {
				return "true", true
			}
			return "false", true
		}
		return _sir_format(recv), true
	case "itself":
		return recv, true
	case "equal?":
		// Ruby `equal?` is object identity.  Immutable primitives compare by
		// value (Ruby interns them); the shared handles (`*Seq`/`*Map`/
		// `*SirInstance`) compare by pointer identity — Go `==` on interface
		// values does exactly this for these cases.  NEVER reflection.
		// Arity guard: `equal?` is newly reachable with zero args via `send`
		// (`obj.send(:equal?)`), so index `args[0]` only after checking length
		// — Ruby raises a catchable `ArgumentError`, not a native Go panic.
		if len(args) == 0 {
			panic(_sir_new_error("ArgumentError",
				Value("wrong number of arguments (given 0, expected 1)")))
		}
		return _sir_value_identical(recv, args[0]), true
	case "respond_to?":
		// M6 honesty: true iff dispatch on `recv` resolves the named method,
		// consulting the SAME reflective/`define_method`/catalog tiers a real
		// call uses (see `_sir_responds_to`).  The name is coerced from a
		// Symbol/string argument and used only as a map/switch KEY — never to
		// reflect a Go method.
		if len(args) == 0 {
			return false, true
		}
		return _sir_responds_to(recv, _sir_method_name(args[0])), true
	case "freeze":
		// v0 has no true immutability — identity-returning, matching Ruby's
		// API shape (`freeze` returns the receiver).
		return recv, true
	case "frozen?":
		// v0: only the always-frozen immutable primitives report frozen.
		switch recv.(type) {
		case nil, bool, int64, int, float64, *Symbol:
			return true, true
		}
		return false, true
	case "dup", "clone":
		// Shallow copy for the mutable handles; primitives are their own dup.
		switch r := recv.(type) {
		case *Seq:
			out := make([]Value, len(r.Items))
			copy(out, r.Items)
			return &Seq{Items: out}, true
		case *Map:
			out := make([]MapEntry, len(r.Entries))
			copy(out, r.Entries)
			return &Map{Entries: out}, true
		}
		return recv, true
	case "to_a":
		// Ruby: `nil.to_a == []`, `Array#to_a == self`; other receivers have
		// no universal `to_a`, so miss and fall through to the honest floor.
		if recv == nil {
			return &Seq{Items: []Value{}}, true
		}
		if s, ok := recv.(*Seq); ok {
			return s, true
		}
		return nil, false
	case "tap":
		// Block-less `tap` (no trailing `Closure` reached `_sir_meta_method`)
		// still returns the receiver — Ruby's Enumerator-less v0 floor.
		return recv, true
	case "then", "yield_self":
		// Block-less `then`/`yield_self` returns the receiver (v0 floor).
		return recv, true
	}
	return nil, false
}

// Object identity for `equal?`.  Immutable primitives (nil/bool/int/float/
// Symbol/string) are compared by VALUE — Ruby interns these, so two `:sym`
// or two `5`s are the same object.  The mutable handles (`*Seq`/`*Map`/
// `*SirInstance`/`*Closure`) are compared by POINTER identity via Go `==`
// on the interface, which is exactly reference identity for pointer types.
func _sir_value_identical(a Value, b Value) bool {
	switch a.(type) {
	case nil, bool, int64, int, float64, string, *Symbol:
		return _sir_value_eq(a, b)
	}
	return a == b
}

// Coerce a `respond_to?`/`send` first argument (a `*Symbol`, a `":m"`-ish
// string, or a bare name) to the plain method name used as the catalog key.
// The result is used ONLY as a switch/map key — never to reflect a Go method
// (the C3 dynamic-dispatch RCE lesson).
func _sir_method_name(arg Value) string {
	switch a := arg.(type) {
	case *Symbol:
		return a.Name
	case string:
		return a
	}
	return _sir_ruby_to_s(arg)
}

// ── M6 universal metaprogramming: send / tap / then / yield_self ───────
//
// These are Ruby Kernel/Object methods that apply to EVERY receiver and are
// special because they either re-enter dispatch with a dynamic name (`send`)
// or drive a trailing block (`tap`/`then`).  They are split out of
// `_sir_object_method` (which is value-returning, block-unaware) because
// `send` recurses through `_sir_call_method` and the block methods need the
// peeled-off `*Closure`.  Dispatched by `_sir_call_method` after the
// type-specific catalogs; a block-less `tap`/`then` MISSES here and falls
// through to `_sir_object_method`'s receiver-identity floor.
//
// SECURITY (the C3 RCE lesson): `send`'s dynamic name is taken from the first
// argument, coerced to a string by `_sir_method_name`, and handed to
// `_sir_call_method` — the SAME explicit catalog/switch a normal call walks.
// An unknown name therefore surfaces the ordinary NoMethodError floor.  The
// name is NEVER used to reflect a Go method/field (no `reflect.MethodByName`).
func _sir_meta_method(recv Value, name string, args []Value) (Value, bool) {
	switch name {
	case "send", "__send__", "public_send":
		// First arg names the method; re-enter dispatch with the remaining
		// args (a trailing block survives as a trailing arg).  An empty arg
		// list bottoms out at the honest floor rather than raising.
		if len(args) == 0 {
			return nil, false
		}
		return _sir_call_method(recv, _sir_method_name(args[0]), args[1:]), true
	case "tap":
		// `tap` yields the receiver to the block and returns the RECEIVER
		// (the "peek in a pipeline" method).  Only with a trailing block;
		// block-less `tap` misses → `_sir_object_method` returns the receiver.
		if _, block := _sir_split_block(args); block != nil {
			_sir_apply(block, []Value{recv})
			return recv, true
		}
		return nil, false
	case "then", "yield_self":
		// `then`/`yield_self` yields the receiver and returns the BLOCK RESULT
		// (functional "pipe into a block").  Block-less → miss → floor returns
		// the receiver.
		if _, block := _sir_split_block(args); block != nil {
			return _sir_apply(block, []Value{recv}), true
		}
		return nil, false
	}
	return nil, false
}

// ── M6 boolean logic: TrueClass/FalseClass `&`/`|`/`^` ─────────────────
//
// Ruby's `&`/`|`/`^` on a boolean are EAGER (non-short-circuiting) logical
// operators — every operand is evaluated, unlike the lazy `&&`/`||`
// keywords — and they coerce the argument by Ruby truthiness (`nil`/`false`
// are falsy, everything else — `0`, `""` — is truthy).  So `true & nil` is
// `false` and `false | 0` is `true`.  `^` is logical XOR.
func _sir_bool_method(recv bool, name string, args []Value) (Value, bool) {
	switch name {
	case "&", "|", "^":
		// Arity guard: these are newly reachable with zero args via `send`
		// (`true.send(:&)`), so require the operand before indexing `args[0]`
		// — Ruby raises a catchable `ArgumentError`, not a native Go panic.
		if len(args) == 0 {
			panic(_sir_new_error("ArgumentError",
				Value("wrong number of arguments (given 0, expected 1)")))
		}
	}
	switch name {
	case "&":
		return recv && _sir_truthy(args[0]), true
	case "|":
		return recv || _sir_truthy(args[0]), true
	case "^":
		return recv != _sir_truthy(args[0]), true
	}
	return nil, false
}

// ── M6 respond_to? — dispatch-honest membership ────────────────────────
//
// Whether dispatch on `recv` resolves `name`, consulting the SAME tiers a
// real call walks: the reflective built-ins, the user `define_method` table
// (for a `*SirInstance`), and the type-specific + universal catalogs.  This
// is the honest discriminator behind `respond_to?` — a catalog method → true,
// an out-of-catalog method → false (and that call returns the NoMethodError
// floor).  `name` is an explicit membership KEY only, never reflection.
func _sir_responds_to(recv Value, name string) bool {
	// Reflective built-ins the frontend/`class` support on every receiver.
	switch name {
	case "is_a?", "kind_of?", "instance_of?", "class":
		return true
	}
	// Universal Object + M6 metaprogramming methods (every receiver).
	if _sir_object_responds(name) {
		return true
	}
	// A user instance: consult its class ancestry method table.
	if inst, ok := recv.(*SirInstance); ok {
		if _, found := _sir_resolve_instance_method(inst.Class, name); found {
			return true
		}
		return false
	}
	// `nil.to_a == []` is a universal method only for the nil receiver.
	if recv == nil && name == "to_a" {
		return true
	}
	// Type-specific catalogs — mirror `_sir_call_method`'s receiver switch.
	switch recv.(type) {
	case string:
		return _sir_string_responds(name)
	case *Symbol:
		return _sir_symbol_responds(name)
	case bool:
		switch name {
		case "&", "|", "^":
			return true
		}
		return false
	case int64, int, float64:
		return _sir_numeric_responds(name)
	case *Seq:
		return _sir_array_responds(name)
	case *Map:
		return _sir_hash_responds(name)
	}
	return false
}

// The universal names resolved on EVERY receiver by `_sir_object_method` +
// `_sir_meta_method`.  Kept in lockstep with those switches.
func _sir_object_responds(name string) bool {
	switch name {
	case "nil?", "==", "!=", "class", "to_s", "inspect", "itself",
		"equal?", "respond_to?", "freeze", "frozen?", "dup", "clone",
		"send", "__send__", "public_send", "tap", "then", "yield_self":
		return true
	case "to_a":
		// `to_a` is universal only for nil/Array; the collection catalogs
		// report it for their own receivers, so keep it out of the universal
		// set and let the type-specific `_sir_*_responds` decide.
		return false
	}
	return false
}

// The following `_sir_*_responds` predicates mirror EXACTLY the `case`
// labels of the matching catalog switch, so `respond_to?` stays honest as
// the catalogs grow.  They list block-taking method names too (a real call
// resolves them when a block is present).
func _sir_string_responds(name string) bool {
	switch name {
	case "length", "size", "upcase", "downcase", "capitalize", "reverse",
		"strip", "lstrip", "rstrip", "chomp", "empty?", "include?",
		"start_with?", "end_with?", "split", "chars", "bytes", "index",
		"replace", "sub", "gsub", "to_i", "to_f", "to_sym",
		"ljust", "rjust", "center", "swapcase",
		"tr", "count", "delete", "squeeze":
		return true
	}
	return false
}

func _sir_symbol_responds(name string) bool {
	switch name {
	case "to_s", "to_sym", "length", "size", "upcase", "downcase", "empty?":
		return true
	}
	return false
}

func _sir_numeric_responds(name string) bool {
	switch name {
	// Block-taking Integer iterators.
	case "times", "upto", "downto", "step":
		return true
	// Non-block Integer/Float methods (kept in lockstep with the
	// `_sir_numeric_method` switch above).
	case "abs", "to_i", "to_int", "to_f", "even?", "odd?", "zero?",
		"positive?", "negative?", "succ", "next", "pred",
		"floor", "ceil", "round", "divmod", "fdiv", "clamp", "between?",
		"gcd", "pow", "**", "digits":
		return true
	}
	return false
}

func _sir_array_responds(name string) bool {
	switch name {
	// Non-block Array methods.
	case "length", "size", "count", "first", "last", "empty?", "include?",
		"index", "push", "append", "<<", "pop", "shift", "reverse", "sort",
		"min", "max", "minmax", "sum", "uniq", "flatten", "compact", "zip", "rotate",
		"to_h", "tally", "take", "drop", "values_at",
		"join", "fetch", "to_a", "each_slice", "each_cons":
		return true
	// Block-taking Array/Enumerable methods.
	case "each", "each_with_index", "map", "collect", "select", "filter",
		"reject", "reduce", "inject", "find", "detect", "any?", "all?", "none?",
		"sort_by", "min_by", "max_by", "group_by", "partition", "flat_map",
		"collect_concat", "take_while", "drop_while", "each_with_object",
		"chunk_while", "slice_when", "cycle":
		return true
	}
	return false
}

func _sir_hash_responds(name string) bool {
	switch name {
	// Non-block Hash methods.
	case "keys", "values", "has_key?", "key?", "include?", "member?",
		"has_value?", "value?", "size", "length", "empty?", "fetch", "to_h":
		return true
	// Block-taking Hash methods.
	case "each", "each_pair", "each_key", "each_value", "map",
		"select", "filter", "reject", "transform_values", "transform_keys",
		"find", "detect", "any?", "all?", "none?", "count",
		"sort_by", "min_by", "max_by",
		"group_by", "partition", "flat_map", "collect_concat",
		"reduce", "inject", "sum",
		"each_with_index", "each_with_object":
		return true
	}
	return false
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
	case "fetch":
		// Ruby `Array#fetch(i)` is the RAISING index read: an out-of-bounds
		// index raises `IndexError` (unlike `arr[i]`, which returns nil).
		// A supplied default (`fetch(i, d)`) is returned instead of raising.
		// Negative indices count from the end (`fetch(-1)` is the last
		// element).  We raise the TYPED `SirError` via `_sir_new_error` +
		// `panic` (the same entry point an explicit `raise` uses), so a
		// translated `rescue IndexError` matches.
		//
		// A NON-integer index (`arr.fetch("x")`) raises a typed, catchable
		// `TypeError` ("no implicit conversion of String into Integer"),
		// matching Ruby — rather than the raw `_sir_as_int` "expected int"
		// panic (which surfaced only as a generic StandardError).
		switch args[0].(type) {
		case int64, int:
			// integer index — proceed
		default:
			panic(_sir_new_error("TypeError",
				Value("no implicit conversion of "+_sir_ruby_class_name(args[0])+" into Integer")))
		}
		i := _sir_as_int(args[0])
		n := int64(len(recv.Items))
		idx := i
		if idx < 0 {
			idx += n
		}
		if idx < 0 || idx >= n {
			if len(args) > 1 {
				return args[1], true
			}
			panic(_sir_new_error("IndexError",
				Value("index "+strconv.FormatInt(i, 10)+" outside of array bounds: "+
					strconv.FormatInt(-n, 10)+"..."+strconv.FormatInt(n, 10))))
		}
		return recv.Items[idx], true
	case "min", "max":
		// Ruby `Array#min`/`#max` (no block, v0): element-wise extremum via
		// `<`/`>` (modelled by `_sir_value_lt`).  Empty array ⇒ nil.  We seed
		// with element 0 and keep the running extremum, matching the TS
		// `reduce` and Python `min`/`max` references.
		if len(recv.Items) == 0 {
			return nil, true
		}
		best := recv.Items[0]
		for _, x := range recv.Items[1:] {
			if name == "min" {
				if _sir_value_lt(x, best) {
					best = x
				}
			} else if _sir_value_lt(best, x) {
				best = x
			}
		}
		return best, true
	case "minmax":
		// Ruby `Array#minmax` (no block): the two-element array `[min, max]` in
		// one pass, via `<` (modelled by `_sir_value_lt`).  An empty array ⇒
		// `[nil, nil]` (no smallest/largest element), matching the Python
		// reference's `[None, None]`.
		if len(recv.Items) == 0 {
			return &Seq{Items: []Value{nil, nil}}, true
		}
		lo := recv.Items[0]
		hi := recv.Items[0]
		for _, x := range recv.Items[1:] {
			if _sir_value_lt(x, lo) {
				lo = x
			}
			if _sir_value_lt(hi, x) {
				hi = x
			}
		}
		return &Seq{Items: []Value{lo, hi}}, true
	case "sum":
		// Ruby `Array#sum`: fold with polymorphic `+` over an initial value
		// (default 0, or the supplied `sum(init)` argument), preserving
		// int/float exactly as `_sir_plus` does.  Empty array ⇒ the initial
		// value (0 by default).  Matches the Python/TS references, which start
		// `total` at `args[0] if args else 0` and accumulate with `+`.
		var acc Value = int64(0)
		if len(args) > 0 {
			acc = args[0]
		}
		for _, x := range recv.Items {
			acc = _sir_plus([]Value{acc, x})
		}
		return acc, true
	case "uniq":
		// Order-preserving de-duplication using structural value-equality
		// (`_sir_value_eq`), matching the reference `_uniq`.  Fresh slice.
		out := []Value{}
		for _, x := range recv.Items {
			dup := false
			for _, y := range out {
				if _sir_value_eq(x, y) {
					dup = true
					break
				}
			}
			if !dup {
				out = append(out, x)
			}
		}
		return &Seq{Items: out}, true
	case "flatten":
		// Recursively flatten nested `*Seq` into a fresh flat `*Seq`.  A
		// `*Seq` is a shared, mutable handle, so a program can build a *cyclic*
		// array (`a = []; a << a`); an unguarded recursive flatten would
		// overflow the Go stack (CWE-674).  We thread a `visited` set of the
		// `*Seq` pointers on the active flatten path — exactly as `_sir_puts`
		// does — and skip a handle already on its own path (a self-cycle
		// contributes nothing, terminating the recursion).  Non-cyclic nested
		// arrays flatten in full because each handle is removed from `visited`
		// on exit, so sibling occurrences are unaffected.
		out := []Value{}
		_sir_flatten_into(&out, recv, make(map[Value]bool))
		return &Seq{Items: out}, true
	case "compact":
		// Fresh array with nil elements removed (Ruby `Array#compact`).
		out := []Value{}
		for _, x := range recv.Items {
			if x != nil {
				out = append(out, x)
			}
		}
		return &Seq{Items: out}, true
	case "zip":
		// `a.zip(b, c, ...)` -> an Array of tuples `[a[i], b[i], ...]`, length =
		// `len(a)`; a shorter operand pads with nil. Non-array operands are
		// treated as empty (pad-only), never raising.
		others := make([]*Seq, 0, len(args))
		for _, o := range args {
			if zs, ok := o.(*Seq); ok {
				others = append(others, zs)
			} else {
				others = append(others, &Seq{Items: nil})
			}
		}
		zipped := make([]Value, len(recv.Items))
		for i, x := range recv.Items {
			tuple := make([]Value, 0, len(others)+1)
			tuple = append(tuple, x)
			for _, o := range others {
				if i < len(o.Items) {
					tuple = append(tuple, o.Items[i])
				} else {
					tuple = append(tuple, nil)
				}
			}
			zipped[i] = &Seq{Items: tuple}
		}
		return &Seq{Items: zipped}, true
	case "rotate":
		// `a.rotate(n=1)` -> elements rotated left by n (negative rotates right);
		// the modulo wraps so any n terminates without panicking.
		n := int64(1)
		if len(args) > 0 {
			n = _sir_as_int_trunc(args[0])
		}
		length := int64(len(recv.Items))
		if length == 0 {
			return &Seq{Items: []Value{}}, true
		}
		shift := ((n % length) + length) % length
		rot := make([]Value, 0, length)
		rot = append(rot, recv.Items[shift:]...)
		rot = append(rot, recv.Items[:shift]...)
		return &Seq{Items: rot}, true
	case "to_h":
		// `[[k, v], ...].to_h` -> a Hash. Each 2-element Array contributes a pair;
		// anything else is skipped (Ruby raises TypeError - deferred to the typed-
		// error cascade; the never-raise floor keeps a controlled result here).
		keys := []Value{}
		vals := []Value{}
		for _, x := range recv.Items {
			if pair, ok := x.(*Seq); ok && len(pair.Items) == 2 {
				keys = append(keys, pair.Items[0])
				vals = append(vals, pair.Items[1])
			}
		}
		return _sir_map_lit(keys, vals), true
	case "tally":
		// `a.tally` -> a Hash of element -> occurrence count, in first-seen order.
		// Keys use structural value-equality (via `_sir_map_get`).
		acc := _sir_map_lit([]Value{}, []Value{})
		for _, x := range recv.Items {
			n := int64(0)
			if c, ok := _sir_map_get(acc, x).(int64); ok {
				n = c
			}
			_sir_map_set(acc, x, n+1)
		}
		return acc, true
	case "take":
		// `a.take(n)` -> a fresh Array of the first n elements. Ruby clamps: n<=0
		// yields [], n>len yields a full copy. A negative n raises ArgumentError in
		// Ruby; the never-raise floor treats it as 0.
		n := int64(0)
		if len(args) > 0 {
			n = _sir_as_int_trunc(args[0])
		}
		if n < 0 {
			n = 0
		}
		if n > int64(len(recv.Items)) {
			n = int64(len(recv.Items))
		}
		out := make([]Value, int(n))
		copy(out, recv.Items[:int(n)])
		return &Seq{Items: out}, true
	case "drop":
		// `a.drop(n)` -> a fresh Array with the first n elements removed (n<=0 -> a
		// full copy, n>=len -> []). A negative n is treated as 0 (never-raise floor).
		n := int64(0)
		if len(args) > 0 {
			n = _sir_as_int_trunc(args[0])
		}
		if n < 0 {
			n = 0
		}
		if n > int64(len(recv.Items)) {
			n = int64(len(recv.Items))
		}
		out := make([]Value, len(recv.Items)-int(n))
		copy(out, recv.Items[int(n):])
		return &Seq{Items: out}, true
	case "values_at":
		// `a.values_at(i, j, ...)` -> a fresh Array of the element at each index,
		// folding a negative index from the end; an out-of-range index yields nil
		// (Ruby's behaviour), never panicking.
		length := int64(len(recv.Items))
		out := make([]Value, 0, len(args))
		for _, a := range args {
			idx := _sir_as_int_trunc(a)
			if idx < 0 {
				idx += length
			}
			if idx >= 0 && idx < length {
				out = append(out, recv.Items[idx])
			} else {
				out = append(out, nil)
			}
		}
		return &Seq{Items: out}, true
	case "to_a":
		return recv, true
	case "each_slice":
		// `each_slice(n)` -> consecutive sub-arrays of at most n elements (the
		// last may be shorter).  `[1,2,3,4,5].each_slice(2)` -> [[1,2],[3,4],[5]].
		// Ruby raises ArgumentError for n <= 0; the never-panic floor yields [].
		n := int64(0)
		if len(args) > 0 {
			n = _sir_as_int_trunc(args[0])
		}
		if n <= 0 {
			return &Seq{Items: []Value{}}, true
		}
		out := []Value{}
		for i := int64(0); i < int64(len(recv.Items)); i += n {
			end := i + n
			if end > int64(len(recv.Items)) {
				end = int64(len(recv.Items))
			}
			slice := make([]Value, end-i)
			copy(slice, recv.Items[i:end])
			out = append(out, &Seq{Items: slice})
		}
		return &Seq{Items: out}, true
	case "each_cons":
		// `each_cons(n)` -> every consecutive n-element sliding window.
		// `[1,2,3,4].each_cons(2)` -> [[1,2],[2,3],[3,4]].  A window larger than
		// the array (or n <= 0) yields [].
		n := int64(0)
		if len(args) > 0 {
			n = _sir_as_int_trunc(args[0])
		}
		out := []Value{}
		if n <= 0 {
			return &Seq{Items: out}, true
		}
		for i := int64(0); i+n <= int64(len(recv.Items)); i++ {
			win := make([]Value, n)
			copy(win, recv.Items[i:i+n])
			out = append(out, &Seq{Items: win})
		}
		return &Seq{Items: out}, true
	}
	return nil, false
}

// Cycle-guarded recursive flatten helper for `Array#flatten`.  Appends the
// leaf (non-`*Seq`) elements of `seq` to `*out` in order, recursing into
// nested `*Seq` handles.  `visited` holds the `*Seq` pointers currently on the
// active recursion path; a handle already present is a self-cycle and is
// skipped rather than recursed into (mirrors `_sir_puts_one`'s guard).
func _sir_flatten_into(out *[]Value, seq *Seq, visited map[Value]bool) {
	if visited[Value(seq)] {
		return
	}
	visited[Value(seq)] = true
	for _, item := range seq.Items {
		if s, ok := item.(*Seq); ok {
			_sir_flatten_into(out, s, visited)
		} else {
			*out = append(*out, item)
		}
	}
	delete(visited, Value(seq))
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
	case "each_with_index":
		// Yields `(element, index)` pairs and returns the receiver, matching
		// the Python/TS `enumerate`-style references.
		for i, x := range recv.Items {
			_sir_apply(block, []Value{x, int64(i)})
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
	case "sort_by":
		// Sort by the block-computed key, stable on ties.  Keys are computed
		// once (Schwartzian).  `_sir_value_lt` is the never-panic comparator.
		type sbKV struct {
			k Value
			v Value
		}
		keyed := make([]sbKV, len(recv.Items))
		for i, x := range recv.Items {
			keyed[i] = sbKV{_sir_apply(block, []Value{x}), x}
		}
		sort.SliceStable(keyed, func(i, j int) bool {
			return _sir_value_lt(keyed[i].k, keyed[j].k)
		})
		out := make([]Value, len(keyed))
		for i := range keyed {
			out[i] = keyed[i].v
		}
		return &Seq{Items: out}, true
	case "min_by", "max_by":
		// Element with the extremal block key (first-on-tie; nil on empty).
		if len(recv.Items) == 0 {
			return nil, true
		}
		wantMin := name == "min_by"
		bestItem := recv.Items[0]
		bestKey := _sir_apply(block, []Value{recv.Items[0]})
		for _, x := range recv.Items[1:] {
			k := _sir_apply(block, []Value{x})
			take := false
			if wantMin {
				take = _sir_value_lt(k, bestKey)
			} else {
				take = _sir_value_lt(bestKey, k)
			}
			if take {
				bestItem = x
				bestKey = k
			}
		}
		return bestItem, true
	case "group_by":
		// A Hash of block key → Array of elements, in first-seen order.
		acc := _sir_map_lit([]Value{}, []Value{})
		for _, x := range recv.Items {
			k := _sir_apply(block, []Value{x})
			if seq, ok := _sir_map_get(acc, k).(*Seq); ok && seq != nil {
				seq.Items = append(seq.Items, x)
			} else {
				_sir_map_set(acc, k, &Seq{Items: []Value{x}})
			}
		}
		return acc, true
	case "partition":
		// `[matching, non_matching]`, each a fresh Array, order preserved.
		yes := []Value{}
		no := []Value{}
		for _, x := range recv.Items {
			if _sir_truthy(_sir_apply(block, []Value{x})) {
				yes = append(yes, x)
			} else {
				no = append(no, x)
			}
		}
		return &Seq{Items: []Value{&Seq{Items: yes}, &Seq{Items: no}}}, true
	case "flat_map", "collect_concat":
		// Map then splice one level: an Array result contributes its elements,
		// a scalar is appended as-is.
		out := []Value{}
		for _, x := range recv.Items {
			r := _sir_apply(block, []Value{x})
			if s, ok := r.(*Seq); ok {
				out = append(out, s.Items...)
			} else {
				out = append(out, r)
			}
		}
		return &Seq{Items: out}, true
	case "take_while":
		out := []Value{}
		for _, x := range recv.Items {
			if _sir_truthy(_sir_apply(block, []Value{x})) {
				out = append(out, x)
			} else {
				break
			}
		}
		return &Seq{Items: out}, true
	case "drop_while":
		out := []Value{}
		dropping := true
		for _, x := range recv.Items {
			if dropping && _sir_truthy(_sir_apply(block, []Value{x})) {
				continue
			}
			dropping = false
			out = append(out, x)
		}
		return &Seq{Items: out}, true
	case "count":
		// `count { |x| pred }` — number of truthy results.
		n := int64(0)
		for _, x := range recv.Items {
			if _sir_truthy(_sir_apply(block, []Value{x})) {
				n++
			}
		}
		return n, true
	case "each_with_object":
		// `each_with_object(memo) { |x, memo| … }` — yields each element with
		// the memo and returns the (mutated) memo.
		if len(args) == 0 {
			return recv, true
		}
		obj := args[0]
		for _, x := range recv.Items {
			_sir_apply(block, []Value{x, obj})
		}
		return obj, true
	case "chunk_while":
		// `chunk_while { |prev, cur| pred }` -> runs of consecutive elements: the
		// block is called on each ADJACENT pair; while it is truthy the run
		// continues, and a falsy result starts a new run.
		// `[1,2,4,5,7].chunk_while { |a,b| b-a==1 }` -> [[1,2],[4,5],[7]].
		// An empty array yields []; a single element yields [[x]].
		if len(recv.Items) == 0 {
			return &Seq{Items: []Value{}}, true
		}
		cur := &Seq{Items: []Value{recv.Items[0]}}
		chunks := []Value{cur}
		for i := 1; i < len(recv.Items); i++ {
			prev := recv.Items[i-1]
			item := recv.Items[i]
			if _sir_truthy(_sir_apply(block, []Value{prev, item})) {
				cur.Items = append(cur.Items, item)
			} else {
				cur = &Seq{Items: []Value{item}}
				chunks = append(chunks, cur)
			}
		}
		return &Seq{Items: chunks}, true
	case "slice_when":
		// `slice_when { |prev, cur| pred }` -> the INVERSE of chunk_while: runs of
		// consecutive elements, starting a NEW run BETWEEN an adjacent pair
		// exactly WHERE the block is truthy (chunk_while starts a new run where
		// the block is FALSY).
		// `[1,2,4,9,10,11,12].slice_when { |a,b| b-a>1 }` -> [[1,2],[4],[9,10,11,12]].
		// An empty array yields []; a single element yields [[x]].
		if len(recv.Items) == 0 {
			return &Seq{Items: []Value{}}, true
		}
		cur := &Seq{Items: []Value{recv.Items[0]}}
		slices := []Value{cur}
		for i := 1; i < len(recv.Items); i++ {
			prev := recv.Items[i-1]
			item := recv.Items[i]
			if _sir_truthy(_sir_apply(block, []Value{prev, item})) {
				cur = &Seq{Items: []Value{item}}
				slices = append(slices, cur)
			} else {
				cur.Items = append(cur.Items, item)
			}
		}
		return &Seq{Items: slices}, true
	case "cycle":
		// `cycle(n) { |x| ... }` -> iterate the array n full passes in order,
		// yielding each element on every pass; always returns nil.
		//
		//   [1,2,3].cycle(2) { |x| out << x }  ->  out == [1,2,3,1,2,3]
		//   [1,2,3].cycle(0) { ... }           ->  no yields, returns nil
		//   [].cycle(5) { ... }                ->  no yields (empty run body)
		//
		// n <= 0, a negative count, an empty receiver, or a nil / non-integer
		// count (Ruby's block-less Enumerator and infinite no-`n` forms) yields
		// nothing rather than hanging, so emitted programs can never spin forever.
		// A boolean count is not an int64/int in Go, so it falls through to nil.
		var n int64
		if len(args) > 0 {
			if iv, ok := args[0].(int64); ok {
				n = iv
			} else if iv, ok := args[0].(int); ok {
				n = int64(iv)
			} else {
				return nil, true
			}
		} else {
			return nil, true
		}
		if n <= 0 {
			return nil, true
		}
		for p := int64(0); p < n; p++ {
			for _, item := range recv.Items {
				_sir_apply(block, []Value{item})
			}
		}
		return nil, true
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
	case "fetch":
		// Ruby `Hash#fetch(key)` is the RAISING lookup: a missing key raises
		// `KeyError` (unlike `hash[key]`, which returns nil).  A supplied
		// default (`fetch(key, d)`) is returned instead of raising.  We
		// raise the TYPED `SirError` via `_sir_new_error` + `panic` (the same
		// entry point an explicit `raise` uses), so a translated `rescue
		// KeyError` (and, since KeyError < IndexError, `rescue IndexError`)
		// matches.
		for _, e := range recv.Entries {
			if _sir_value_eq(e.Key, args[0]) {
				return e.Val, true
			}
		}
		if len(args) > 1 {
			return args[1], true
		}
		panic(_sir_new_error("KeyError", Value("key not found: "+_sir_format(args[0]))))
	case "to_a":
		// Ruby `Hash#to_a` → an Array of `[key, value]` two-element Arrays,
		// in insertion order.  Each pair is its own fresh `*Seq`, so mutating
		// one never touches the map's backing entries.
		out := make([]Value, len(recv.Entries))
		for i, e := range recv.Entries {
			out[i] = &Seq{Items: []Value{e.Key, e.Val}}
		}
		return &Seq{Items: out}, true
	case "to_h":
		// Ruby `Hash#to_h` with NO block → a shallow copy of the hash (Ruby
		// returns `self`; a fresh `*Map` matches the value semantics without
		// aliasing the receiver's entries).  The block form (which re-maps each
		// pair to a new `[k, v]`) lives in `_sir_hash_block_method`.
		keys := make([]Value, len(recv.Entries))
		vals := make([]Value, len(recv.Entries))
		for i, e := range recv.Entries {
			keys[i] = e.Key
			vals[i] = e.Val
		}
		return _sir_map_lit(keys, vals), true
	case "dig":
		// Ruby `Hash#dig(k, …)` — a NESTED lookup that walks one key per
		// argument, returning nil the moment a level is missing (never
		// raising).  A single argument degrades to a plain lookup, matching
		// the Python/TS reference's single-level `dig`; extra arguments
		// recurse into a nested `*Map` (or `*Seq`, via `Array#dig`) — Ruby
		// digs through anything that itself responds to `dig`.
		var cur Value = recv
		for _, k := range args {
			switch node := cur.(type) {
			case *Map:
				found := false
				for _, e := range node.Entries {
					if _sir_value_eq(e.Key, k) {
						cur = e.Val
						found = true
						break
					}
				}
				if !found {
					return nil, true
				}
			case *Seq:
				// `Array#dig` indexes by an integer key; anything else (or an
				// out-of-range index) digs to nil rather than raising.
				idx, ok := _sir_dig_index(k, len(node.Items))
				if !ok {
					return nil, true
				}
				cur = node.Items[idx]
			default:
				// The current level cannot be dug into (e.g. an Integer with
				// keys still remaining) ⇒ nil, matching Ruby's TypeError-free
				// never-raise floor on the OO surface.
				return nil, true
			}
		}
		return cur, true
	case "store", "[]=":
		// Ruby `Hash#store(k, v)` / `h[k] = v` — an in-place upsert that
		// returns the stored value.  Overwrite an existing key in place;
		// otherwise append, preserving insertion order.
		for i, e := range recv.Entries {
			if _sir_value_eq(e.Key, args[0]) {
				recv.Entries[i].Val = args[1]
				return args[1], true
			}
		}
		recv.Entries = append(recv.Entries, MapEntry{Key: args[0], Val: args[1]})
		return args[1], true
	case "merge":
		// Ruby `Hash#merge(other)` → a FRESH hash (self is NOT mutated).
		// Self's entries come first (insertion order preserved); `other`
		// then overwrites colliding keys in place and appends new ones.  We
		// build a brand-new `[]MapEntry` so the result never aliases either
		// input's backing slice.
		m := &Map{Entries: make([]MapEntry, len(recv.Entries))}
		copy(m.Entries, recv.Entries)
		if other, ok := args[0].(*Map); ok {
			for _, oe := range other.Entries {
				replaced := false
				for i, e := range m.Entries {
					if _sir_value_eq(e.Key, oe.Key) {
						m.Entries[i].Val = oe.Val
						replaced = true
						break
					}
				}
				if !replaced {
					m.Entries = append(m.Entries, MapEntry{Key: oe.Key, Val: oe.Val})
				}
			}
		}
		return m, true
	case "delete":
		// Ruby `Hash#delete(k)` — remove the entry IN PLACE and return its
		// value, or nil when the key was absent.  We rebuild the slice
		// without the matched index rather than shifting in place so the
		// result stays a clean, insertion-ordered `[]MapEntry`.
		for i, e := range recv.Entries {
			if _sir_value_eq(e.Key, args[0]) {
				val := e.Val
				recv.Entries = append(recv.Entries[:i], recv.Entries[i+1:]...)
				return val, true
			}
		}
		return nil, true
	case "clear":
		// Ruby `Hash#clear` — empty self IN PLACE and return self.  A fresh
		// empty slice (not `nil`) keeps later appends well-defined.
		recv.Entries = []MapEntry{}
		return recv, true
	case "invert":
		// Ruby `Hash#invert` → a FRESH hash with keys and values swapped.
		// On duplicate original values the LAST wins (Ruby's documented
		// behaviour), so a later collision overwrites the earlier entry in
		// place.  The result is a brand-new `*Map` — no aliasing of self.
		m := &Map{Entries: []MapEntry{}}
		for _, e := range recv.Entries {
			replaced := false
			for i, ne := range m.Entries {
				if _sir_value_eq(ne.Key, e.Val) {
					m.Entries[i].Val = e.Key
					replaced = true
					break
				}
			}
			if !replaced {
				m.Entries = append(m.Entries, MapEntry{Key: e.Val, Val: e.Key})
			}
		}
		return m, true
	}
	return nil, false
}

// Coerce a `Hash#dig` / `Array#dig` step key into a valid `*Seq` index.
// Ruby indexes arrays by Integer, allowing Python-style negatives that count
// from the end.  Returns the normalised in-range index and true, or false
// when the key is not an integer or lands out of range (⇒ the dig yields
// nil rather than raising).
func _sir_dig_index(k Value, length int) (int, bool) {
	var idx int
	switch n := k.(type) {
	case int64:
		idx = int(n)
	case int:
		idx = n
	default:
		return 0, false
	}
	if idx < 0 {
		idx += length
	}
	if idx < 0 || idx >= length {
		return 0, false
	}
	return idx, true
}

func _sir_hash_block_method(recv *Map, name string, args []Value, block *Closure) (Value, bool) {
	switch name {
	case "each", "each_pair":
		for _, e := range recv.Entries {
			_sir_apply(block, []Value{e.Key, e.Val})
		}
		return recv, true
	case "each_key":
		// Ruby `Hash#each_key` yields ONE argument (the key) per entry and
		// returns self.
		for _, e := range recv.Entries {
			_sir_apply(block, []Value{e.Key})
		}
		return recv, true
	case "each_value":
		// Ruby `Hash#each_value` yields ONE argument (the value) per entry
		// and returns self.
		for _, e := range recv.Entries {
			_sir_apply(block, []Value{e.Val})
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
	case "transform_values":
		// Ruby `Hash#transform_values` yields ONE argument (the value) per
		// entry and builds a NEW hash whose keys are untouched and whose
		// values are the block results. Because the keys are copied verbatim
		// and stay distinct, no collision can occur — a straight append keeps
		// the original insertion order.
		m := &Map{Entries: make([]MapEntry, 0, len(recv.Entries))}
		for _, e := range recv.Entries {
			m.Entries = append(m.Entries, MapEntry{Key: e.Key, Val: _sir_apply(block, []Value{e.Val})})
		}
		return m, true
	case "transform_keys":
		// Ruby `Hash#transform_keys` yields ONE argument (the key) per entry
		// and builds a NEW hash whose values are untouched and whose keys are
		// the block results. Two source keys can map to the SAME new key; Ruby
		// keeps the LAST such entry's value, so we route every write through
		// `_sir_map_put`, which overwrites an existing key in place.
		m := &Map{Entries: make([]MapEntry, 0, len(recv.Entries))}
		for _, e := range recv.Entries {
			_sir_map_put(m, _sir_apply(block, []Value{e.Key}), e.Val)
		}
		return m, true
	// ── Enumerable aggregates (Hash includes Enumerable) ───────────
	//
	// Ruby's Hash mixes in Enumerable, so these iterate the hash as a
	// sequence of [key, value] pairs: the block is yielded (key, value)
	// (two arguments, matching `each`), and the "element" an aggregate
	// returns is the two-element [key, value] Array (`&Seq{key, value}`).
	case "find", "detect":
		// First [k, v] pair whose block result is truthy; nil if none.
		for _, e := range recv.Entries {
			if _sir_truthy(_sir_apply(block, []Value{e.Key, e.Val})) {
				return &Seq{Items: []Value{e.Key, e.Val}}, true
			}
		}
		return nil, true
	case "any?":
		for _, e := range recv.Entries {
			if _sir_truthy(_sir_apply(block, []Value{e.Key, e.Val})) {
				return true, true
			}
		}
		return false, true
	case "all?":
		for _, e := range recv.Entries {
			if !_sir_truthy(_sir_apply(block, []Value{e.Key, e.Val})) {
				return false, true
			}
		}
		return true, true
	case "none?":
		for _, e := range recv.Entries {
			if _sir_truthy(_sir_apply(block, []Value{e.Key, e.Val})) {
				return false, true
			}
		}
		return true, true
	case "count":
		// count { |k, v| pred } — number of pairs with a truthy block result.
		n := int64(0)
		for _, e := range recv.Entries {
			if _sir_truthy(_sir_apply(block, []Value{e.Key, e.Val})) {
				n++
			}
		}
		return n, true
	case "sort_by":
		// A NEW Array of [k, v] pairs sorted by the block key, stable on ties.
		// Keys are computed once (Schwartzian); `_sir_value_lt` never panics.
		type sbKV struct {
			key  Value
			pair Value
		}
		keyed := make([]sbKV, len(recv.Entries))
		for i, e := range recv.Entries {
			keyed[i] = sbKV{_sir_apply(block, []Value{e.Key, e.Val}), &Seq{Items: []Value{e.Key, e.Val}}}
		}
		sort.SliceStable(keyed, func(i, j int) bool {
			return _sir_value_lt(keyed[i].key, keyed[j].key)
		})
		out := make([]Value, len(keyed))
		for i := range keyed {
			out[i] = keyed[i].pair
		}
		return &Seq{Items: out}, true
	case "min_by", "max_by":
		// The [k, v] pair with the extremal block key (first-on-tie; nil on
		// an empty hash).
		if len(recv.Entries) == 0 {
			return nil, true
		}
		wantMin := name == "min_by"
		best := recv.Entries[0]
		bestKey := _sir_apply(block, []Value{best.Key, best.Val})
		for _, e := range recv.Entries[1:] {
			k := _sir_apply(block, []Value{e.Key, e.Val})
			take := false
			if wantMin {
				take = _sir_value_lt(k, bestKey)
			} else {
				take = _sir_value_lt(bestKey, k)
			}
			if take {
				best = e
				bestKey = k
			}
		}
		return &Seq{Items: []Value{best.Key, best.Val}}, true
	// ── Enumerable breadth (grouping / folding / flattening) ───────
	//
	// The block is yielded (key, value) two args (except `reduce`/`inject`,
	// which follow Ruby's memo convention and yield (memo, [key, value]) — the
	// pair as ONE second argument).  Every "element" a result carries is the
	// two-element [key, value] Array (`&Seq{key, value}`).
	case "group_by":
		// A Hash of block key → Array of [k, v] pairs, in first-seen key order.
		acc := _sir_map_lit([]Value{}, []Value{})
		for _, e := range recv.Entries {
			k := _sir_apply(block, []Value{e.Key, e.Val})
			pair := &Seq{Items: []Value{e.Key, e.Val}}
			if seq, ok := _sir_map_get(acc, k).(*Seq); ok && seq != nil {
				seq.Items = append(seq.Items, pair)
			} else {
				_sir_map_set(acc, k, &Seq{Items: []Value{pair}})
			}
		}
		return acc, true
	case "partition":
		// `[matching pairs, non-matching pairs]`, order preserved.
		yes := []Value{}
		no := []Value{}
		for _, e := range recv.Entries {
			pair := &Seq{Items: []Value{e.Key, e.Val}}
			if _sir_truthy(_sir_apply(block, []Value{e.Key, e.Val})) {
				yes = append(yes, pair)
			} else {
				no = append(no, pair)
			}
		}
		return &Seq{Items: []Value{&Seq{Items: yes}, &Seq{Items: no}}}, true
	case "flat_map", "collect_concat":
		// Map each pair through the block, splicing one level: an Array result
		// contributes its elements, a scalar is appended as-is.
		out := []Value{}
		for _, e := range recv.Entries {
			r := _sir_apply(block, []Value{e.Key, e.Val})
			if s, ok := r.(*Seq); ok {
				out = append(out, s.Items...)
			} else {
				out = append(out, r)
			}
		}
		return &Seq{Items: out}, true
	case "reduce", "inject":
		// `reduce(init) { |memo, (k, v)| … }` folds the pairs; a seedless
		// `reduce` starts from the first pair.  The block yields the pair as ONE
		// second argument.  Empty seedless reduce ⇒ nil.
		pairs := make([]Value, len(recv.Entries))
		for i, e := range recv.Entries {
			pairs[i] = &Seq{Items: []Value{e.Key, e.Val}}
		}
		var acc Value
		var rest []Value
		if len(args) > 0 {
			acc = args[0]
			rest = pairs
		} else if len(pairs) > 0 {
			acc = pairs[0]
			rest = pairs[1:]
		} else {
			return nil, true
		}
		for _, pair := range rest {
			acc = _sir_apply(block, []Value{acc, pair})
		}
		return acc, true
	case "sum":
		// `sum(init = 0) { |k, v| … }` — `init` plus the polymorphic-`+` sum of
		// the block results (Hash#sum requires a block).
		var acc Value = int64(0)
		if len(args) > 0 {
			acc = args[0]
		}
		for _, e := range recv.Entries {
			acc = _sir_plus([]Value{acc, _sir_apply(block, []Value{e.Key, e.Val})})
		}
		return acc, true
	case "to_h":
		// `Hash#to_h { |k, v| [new_k, new_v] }` — a NEW hash from the `[k, v]`
		// pairs the block returns.  The block is yielded the two args `(k, v)`
		// (matching `each`) and must return a 2-element `*Seq`; a non-pair result
		// is skipped (the never-raise floor — Ruby raises TypeError, deferred to
		// the typed-error cascade), and a later pair with a duplicate key wins
		// (Ruby's rule, and how `_sir_map_set` already behaves).
		acc := _sir_map_lit([]Value{}, []Value{})
		for _, e := range recv.Entries {
			r := _sir_apply(block, []Value{e.Key, e.Val})
			if pair, ok := r.(*Seq); ok && len(pair.Items) == 2 {
				_sir_map_set(acc, pair.Items[0], pair.Items[1])
			}
		}
		return acc, true
	case "each_with_index":
		// `each_with_index { |(k, v), i| … }` — yields each `[k, v]` pair with
		// its 0-based index and returns the receiver.  Unlike the two-arg
		// `(k, v)` yield of `each`, the element arrives as a single `[k, v]`
		// `*Seq` (the second block param is the index), matching Ruby's
		// Enumerable convention.
		for i, e := range recv.Entries {
			_sir_apply(block, []Value{&Seq{Items: []Value{e.Key, e.Val}}, int64(i)})
		}
		return recv, true
	case "each_with_object":
		// `each_with_object(memo) { |(k, v), memo| … }` — yields each `[k, v]`
		// pair with the memo object and returns the (mutated) memo.  Like
		// `each_with_index`, the element is the single `[k, v]` pair (the second
		// block param is the memo).  With no memo argument the receiver is
		// returned unchanged.
		if len(args) == 0 {
			return recv, true
		}
		memo := args[0]
		for _, e := range recv.Entries {
			_sir_apply(block, []Value{&Seq{Items: []Value{e.Key, e.Val}}, memo})
		}
		return memo, true
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
	case "capitalize":
		// Ruby `String#capitalize`: first character upcased, the rest
		// downcased (e.g. `"hELLO".capitalize == "Hello"`).  We work on
		// `[]rune` so a multibyte first character upcases correctly and we
		// never split a UTF-8 sequence mid-codepoint.
		r := []rune(recv)
		if len(r) == 0 {
			return "", true
		}
		head := strings.ToUpper(string(r[0]))
		tail := strings.ToLower(string(r[1:]))
		return head + tail, true
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
	case "chomp":
		// Ruby `String#chomp`: with an explicit separator argument, drop
		// exactly that trailing suffix (once); with no argument, drop one
		// trailing "\r\n", "\n", or "\r" (Ruby's default record separator).
		if len(args) > 0 {
			if sep, ok := args[0].(string); ok {
				if sep != "" && strings.HasSuffix(recv, sep) {
					return recv[:len(recv)-len(sep)], true
				}
				return recv, true
			}
			return recv, true
		}
		if strings.HasSuffix(recv, "\r\n") {
			return recv[:len(recv)-2], true
		}
		if strings.HasSuffix(recv, "\n") || strings.HasSuffix(recv, "\r") {
			return recv[:len(recv)-1], true
		}
		return recv, true
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
	case "bytes":
		// Ruby `String#bytes`: the raw UTF-8 byte values as integers.  A Go
		// `string` is already UTF-8, so we iterate its bytes directly.
		b := []byte(recv)
		out := make([]Value, len(b))
		for i, x := range b {
			out[i] = int64(x)
		}
		return &Seq{Items: out}, true
	case "index":
		// Ruby `String#index`: the rune index of the first occurrence of the
		// substring, or `nil` when absent.  `strings.Index` reports a *byte*
		// offset, so we convert it to a rune count to match Ruby's character
		// indexing on multibyte strings.
		if len(args) > 0 {
			if s, ok := args[0].(string); ok {
				bi := strings.Index(recv, s)
				if bi < 0 {
					return nil, true
				}
				return int64(len([]rune(recv[:bi]))), true
			}
		}
		return nil, true
	case "replace":
		// Ruby `String#replace` overwrites the entire content; for an
		// immutable Go string that is simply the replacement value.
		if len(args) > 0 {
			if s, ok := args[0].(string); ok {
				return s, true
			}
		}
		return recv, true
	case "sub":
		// Ruby `String#sub` with string arguments: literal replacement of the
		// FIRST occurrence only (n == 1).  `strings.Replace` inserts the
		// replacement verbatim — no `$&`/`\1` back-reference expansion.
		if len(args) >= 2 {
			from, ok1 := args[0].(string)
			to, ok2 := args[1].(string)
			if ok1 && ok2 {
				return strings.Replace(recv, from, to, 1), true
			}
		}
		return recv, true
	case "gsub":
		// Ruby `String#gsub` with string arguments: literal replacement of
		// ALL occurrences.  `strings.ReplaceAll` is verbatim (no regex, no
		// back-reference expansion).
		if len(args) >= 2 {
			from, ok1 := args[0].(string)
			to, ok2 := args[1].(string)
			if ok1 && ok2 {
				return strings.ReplaceAll(recv, from, to), true
			}
		}
		return recv, true
	case "to_i":
		return _sir_str_to_i(recv), true
	case "to_f":
		return _sir_str_to_f(recv), true
	case "to_sym":
		return _sir_intern(recv), true
	case "ljust", "rjust", "center":
		// Ruby String#ljust/#rjust/#center(width, pad = " "): pad to `width`
		// RUNES using `pad` cyclically.  width <= the current rune length
		// returns the string unchanged; center puts any odd extra pad on the
		// RIGHT (Ruby's rule).  An empty pad degrades to a single space rather
		// than raising, holding the never-raise floor.
		width := int64(0)
		if len(args) > 0 {
			width = _sir_as_int_trunc(args[0])
		}
		pad := " "
		if len(args) > 1 {
			if p, ok := args[1].(string); ok && p != "" {
				pad = p
			}
		}
		cur := int64(len([]rune(recv)))
		if width <= cur {
			return recv, true
		}
		total := int(width - cur)
		switch name {
		case "ljust":
			return recv + _sir_str_pad(pad, total), true
		case "rjust":
			return _sir_str_pad(pad, total) + recv, true
		default: // center
			left := total / 2
			return _sir_str_pad(pad, left) + recv + _sir_str_pad(pad, total-left), true
		}
	case "swapcase":
		// Ruby String#swapcase: flip the case of each ASCII letter (leaving
		// non-letters and non-ASCII runes untouched).  Works on []rune so a
		// multibyte string is never split mid-codepoint.
		r := []rune(recv)
		for i, c := range r {
			if c >= 'A' && c <= 'Z' {
				r[i] = c + 32
			} else if c >= 'a' && c <= 'z' {
				r[i] = c - 32
			}
		}
		return string(r), true
	case "tr":
		// Ruby String#tr(from, to): position-wise rune translation.  A shorter
		// `to` repeats its last rune; an empty `to` deletes matching runes; a
		// repeated rune in `from` keeps the last mapping.  Literal only — the
		// range (`"a-z"`) and negation (`"^abc"`) forms are a follow-up, matching
		// the literal-only sub/gsub precedent here.
		if len(args) < 2 {
			return recv, true
		}
		from, fok := args[0].(string)
		to, tok := args[1].(string)
		if !fok || !tok {
			return recv, true
		}
		toR := []rune(to)
		table := make(map[rune]rune)
		del := make(map[rune]bool)
		for i, c := range []rune(from) {
			if len(toR) == 0 {
				del[c] = true
				delete(table, c)
			} else if i < len(toR) {
				table[c] = toR[i]
				delete(del, c)
			} else {
				table[c] = toR[len(toR)-1]
				delete(del, c)
			}
		}
		out := make([]rune, 0, len(recv))
		for _, c := range recv {
			if del[c] {
				continue
			}
			if r, ok := table[c]; ok {
				out = append(out, r)
			} else {
				out = append(out, c)
			}
		}
		return string(out), true
	case "count", "delete", "squeeze":
		// Char-set methods.  Each `set` argument is treated LITERALLY — the runes
		// it contains (ranges/negation are a follow-up).  `count` tallies runes of
		// `recv` in the set; `delete` removes them; `squeeze` collapses consecutive
		// runs (of set runes, or of ALL runes when no set is given).  Multiple set
		// args intersect (Ruby's rule).
		sets := make([]map[rune]bool, 0, len(args))
		for _, a := range args {
			if s, ok := a.(string); ok {
				m := make(map[rune]bool)
				for _, c := range s {
					m[c] = true
				}
				sets = append(sets, m)
			}
		}
		inAll := func(c rune) bool {
			if len(sets) == 0 {
				return false
			}
			for _, m := range sets {
				if !m[c] {
					return false
				}
			}
			return true
		}
		if name == "squeeze" && len(sets) == 0 {
			out := make([]rune, 0, len(recv))
			for _, c := range recv {
				if len(out) == 0 || out[len(out)-1] != c {
					out = append(out, c)
				}
			}
			return string(out), true
		}
		if name == "count" {
			n := int64(0)
			for _, c := range recv {
				if inAll(c) {
					n++
				}
			}
			return n, true
		}
		if name == "delete" {
			out := make([]rune, 0, len(recv))
			for _, c := range recv {
				if !inAll(c) {
					out = append(out, c)
				}
			}
			return string(out), true
		}
		out := make([]rune, 0, len(recv))
		for _, c := range recv {
			if len(out) > 0 && out[len(out)-1] == c && inAll(c) {
				continue
			}
			out = append(out, c)
		}
		return string(out), true
	}
	return nil, false
}

// _sir_str_pad builds a padding string of exactly `n` runes by repeating
// `pad` cyclically (truncating the final repeat).  `n <= 0` or an empty pad
// yields "" — callers guarantee a non-empty pad, so this is purely defensive.
func _sir_str_pad(pad string, n int) string {
	if n <= 0 || pad == "" {
		return ""
	}
	pr := []rune(pad)
	out := make([]rune, 0, n)
	for len(out) < n {
		out = append(out, pr[len(out)%len(pr)])
	}
	return string(out)
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
	// Block-taking methods are dispatched when a trailing *Closure is present:
	// `times`/`upto`/`downto`/`step` each iterate a block and return the
	// receiver (Ruby's Integer iterators).  Parity with the Python/TS
	// `_numeric_block_method` catalog.  `positional` is the arg list with the
	// trailing block stripped off (empty for `times`, one limit for
	// `upto`/`downto`, limit + optional stride for `step`).
	positional, block := _sir_split_block(args)
	if block != nil {
		switch name {
		case "times":
			// `n.times { |i| … }` yields 0,1,…,n-1.  A non-positive `n` yields
			// nothing (the loop condition is immediately false).
			n := _sir_as_int_trunc(recv)
			for i := int64(0); i < n; i++ {
				_sir_apply(block, []Value{i})
			}
			return recv, true
		case "upto":
			// `a.upto(b) { |i| … }` yields a,a+1,…,b (inclusive); no iterations
			// when a > b.  Uses truncating int coercion so a float endpoint
			// behaves like the reference (`3.upto(5.9)` stops at 5).
			if len(positional) >= 1 {
				lo := _sir_as_int_trunc(recv)
				hi := _sir_as_int_trunc(positional[0])
				// Guard the terminal `i++` so a finite `hi == MaxInt64` limit
				// terminates instead of wrapping to MinInt64 and spinning.
				for i := lo; i <= hi; {
					_sir_apply(block, []Value{i})
					if i == math.MaxInt64 {
						break
					}
					i++
				}
				return recv, true
			}
		case "downto":
			// `a.downto(b) { |i| … }` yields a,a-1,…,b (inclusive); no
			// iterations when a < b.
			if len(positional) >= 1 {
				hi := _sir_as_int_trunc(recv)
				lo := _sir_as_int_trunc(positional[0])
				for i := hi; i >= lo; {
					_sir_apply(block, []Value{i})
					if i == math.MinInt64 {
						break
					}
					i--
				}
				return recv, true
			}
		case "step":
			// `a.step(limit, stride) { |v| … }` yields a, a+stride, … while
			// `v <= limit` (positive stride) or `v >= limit` (negative stride).
			// A float receiver/limit/stride runs the whole walk in float64;
			// an all-integer walk stays exact.  A zero stride yields nothing
			// (rather than spinning forever) — the never-hang floor.
			if len(positional) >= 1 {
				stride := Value(int64(1))
				if len(positional) >= 2 {
					stride = positional[1]
				}
				limit := positional[0]
				useFloat := _sir_is_float_val(recv) || _sir_is_float_val(limit) || _sir_is_float_val(stride)
				if useFloat {
					step := _sir_as_float_lenient(stride)
					lim := _sir_as_float_lenient(limit)
					v := _sir_as_float(recv)
					if step > 0 {
						for v <= lim {
							_sir_apply(block, []Value{v})
							if v > math.MaxInt64-step {
								break
							}
							prev := v
							v += step
							if v == prev {
								// float stagnation (ulp >= step): the never-hang floor.
								break
							}
						}
					} else if step < 0 {
						for v >= lim {
							_sir_apply(block, []Value{v})
							if v < math.MinInt64-step {
								break
							}
							prev := v
							v += step
							if v == prev {
								// float stagnation (ulp >= step): the never-hang floor.
								break
							}
						}
					}
				} else {
					step := _sir_as_int_trunc(stride)
					lim := _sir_as_int_trunc(limit)
					v := _sir_as_int(recv)
					if step > 0 {
						for v <= lim {
							_sir_apply(block, []Value{v})
							if v > math.MaxInt64-step {
								break
							}
							v += step
						}
					} else if step < 0 {
						for v >= lim {
							_sir_apply(block, []Value{v})
							if v < math.MinInt64-step {
								break
							}
							v += step
						}
					}
				}
				return recv, true
			}
		}
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
	case "to_i", "to_int":
		// Ruby's `to_i`/`to_int` truncate a float toward zero (`3.7.to_i == 3`,
		// `(-3.7).to_i == -3`); on an integer they are the identity.
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
	case "floor":
		// `floor` returns the greatest integer ≤ self.  On an integer it is
		// the identity; on a float it rounds toward −∞ and yields an integer
		// (Ruby: `3.7.floor == 3`, `(-3.2).floor == -4`).  A non-finite float
		// degrades to 0 via `_sir_as_int_trunc` (never-raise floor).
		if isInt {
			return _sir_as_int(recv), true
		}
		f := _sir_as_float(recv)
		if math.IsNaN(f) || math.IsInf(f, 0) {
			return int64(0), true
		}
		return int64(math.Floor(f)), true
	case "ceil":
		// `ceil` returns the least integer ≥ self.  Identity on an integer;
		// rounds a float toward +∞ (`3.2.ceil == 4`, `(-3.7).ceil == -3`).
		if isInt {
			return _sir_as_int(recv), true
		}
		f := _sir_as_float(recv)
		if math.IsNaN(f) || math.IsInf(f, 0) {
			return int64(0), true
		}
		return int64(math.Ceil(f)), true
	case "round":
		// Ruby `round` / `round(ndigits)` — half AWAY from zero (unlike Go's
		// `math.Round`, routed through `_sir_ruby_round` to stay in lockstep
		// with the Python/TS reference: `2.5.round == 3`, `(-2.5).round == -3`).
		// With no argument (or `ndigits >= 0` on an Integer) the result is an
		// integer; a positive `ndigits` on a Float rounds to that many decimals;
		// `ndigits <= 0` rounds to a power of ten.  A non-finite float degrades
		// to the receiver/0.  Go's int64/float64 are FIXED width (no bignum), so
		// the only guard needed is against `10^k` overflowing int64: a place
		// count past int64's ~18 decimal digits dwarfs the value ⇒ 0 (Ruby
		// `1234.round(-30) == 0`), and a large positive `ndigits` past float
		// precision returns the value unchanged.
		ndigits := int64(0)
		if len(args) > 0 {
			ndigits = _sir_as_int_trunc(args[0])
		}
		if isInt {
			iv := _sir_as_int(recv)
			if ndigits >= 0 {
				return iv, true
			}
			if -ndigits > 18 {
				return int64(0), true
			}
			factor := _sir_pow10(-ndigits)
			return _sir_round_int_to_multiple(iv, factor), true
		}
		f := _sir_as_float(recv)
		if math.IsNaN(f) || math.IsInf(f, 0) {
			return int64(0), true
		}
		if ndigits <= 0 {
			if -ndigits > 18 {
				return int64(0), true
			}
			factor := _sir_pow10(-ndigits)
			return _sir_round_int_to_multiple(_sir_ruby_round(f), factor), true
		}
		if ndigits > 17 {
			return f, true // already at full Float precision
		}
		scale := math.Pow(10, float64(ndigits))
		scaled := f * scale
		if math.IsInf(scaled, 0) {
			return f, true // overflow guard: no fractional part left to round
		}
		return float64(_sir_ruby_round(scaled)) / scale, true
	case "divmod":
		// Ruby `divmod(n)` → `[quotient, remainder]` with a FLOORED quotient and
		// the divisor-signed remainder.  Division by zero raises a typed
		// `ZeroDivisionError` (so a translated `rescue` matches).  Int/int uses
		// exact integer math; a float operand promotes to float64 (Go float
		// division of a nonzero-divided-by-nonzero never panics).
		if len(args) < 1 {
			return nil, false
		}
		divIsInt := _sir_is_int(args[0])
		if isInt && divIsInt {
			d := _sir_as_int(args[0])
			if d == 0 {
				panic(_sir_new_error("ZeroDivisionError", Value("divided by 0")))
			}
			n := _sir_as_int(recv)
			q := _sir_floor_div(n, d)
			r := n - q*d
			return &Seq{Items: []Value{q, r}}, true
		}
		df := _sir_as_float(args[0])
		if df == 0 {
			panic(_sir_new_error("ZeroDivisionError", Value("divided by 0")))
		}
		nf := _sir_as_float(recv)
		q := math.Floor(nf / df)
		r := nf - q*df
		return &Seq{Items: []Value{q, r}}, true
	case "fdiv":
		// Ruby `fdiv(n)`: floating-point division that NEVER raises — dividing
		// by zero yields ±Infinity/NaN (Go float division already produces these
		// rather than panicking), honouring the never-raise floor.
		if len(args) < 1 {
			return nil, false
		}
		return _sir_as_float(recv) / _sir_as_float(args[0]), true
	case "clamp":
		// Ruby `Comparable#clamp(min, max)`: `min` if recv < min, `max` if
		// recv > max, else recv.  Compared numerically (float view) so mixed
		// int/float bounds behave; the original receiver value is returned
		// unchanged when in range.  (The Range form is deferred.)
		if len(args) < 2 {
			return nil, false
		}
		rv := _sir_as_float(recv)
		if rv < _sir_as_float(args[0]) {
			return args[0], true
		}
		if rv > _sir_as_float(args[1]) {
			return args[1], true
		}
		return recv, true
	case "between?":
		// Ruby `Comparable#between?(min, max)`: `min <= recv <= max`.
		if len(args) < 2 {
			return nil, false
		}
		rv := _sir_as_float(recv)
		return rv >= _sir_as_float(args[0]) && rv <= _sir_as_float(args[1]), true
	case "gcd":
		// `a.gcd(b)` is the (non-negative) greatest common divisor, via
		// Euclid on the truncated magnitudes (matching Python `math.gcd` and
		// the TS `gcdInt`).  `0.gcd(0) == 0`.  Requires one argument; a
		// missing arg is the controlled arity floor.
		if len(args) < 1 {
			return nil, false
		}
		return _sir_gcd(_sir_as_int_trunc(recv), _sir_as_int_trunc(args[0])), true
	case "pow", "**":
		// `base.pow(exp)` / `base ** exp`.  Requires one argument.  Integer
		// base AND exponent stay in the exact integer tower (int64 wrapping,
		// the SAME convention as `_sir_times`), guarded so a hostile exponent
		// cannot spin an unbounded loop; any float operand promotes to
		// float64 `math.Pow`.  See `_sir_int_pow`.
		if len(args) < 1 {
			return nil, false
		}
		if isInt {
			if e, ok := _sir_int_val(args[0]); ok {
				return _sir_int_pow(_sir_as_int(recv), e), true
			}
		}
		// `recv` is numeric on this dispatch path; a non-numeric exponent
		// degrades to 0.0 (→ result 1.0) via the lenient coercion rather than
		// panicking on the dispatch surface.
		return math.Pow(_sir_as_float(recv), _sir_as_float_lenient(args[0])), true
	case "digits":
		// Ruby `Integer#digits`: the base-10 digits, LEAST-significant first
		// (`123.digits == [3, 2, 1]`).  A float receiver truncates first
		// (parity with the reference, which coerces via `int(recv)`).  The
		// magnitude is taken so a negative receiver produces its digits
		// (Ruby raises `Math::DomainError` on a true negative, but the
		// reference runtimes take the absolute value — we match them).
		return _sir_digits(_sir_as_int_trunc(recv)), true
	}
	return nil, false
}

func _sir_int_val(v Value) (int64, bool) {
	switch n := v.(type) {
	case int64:
		return n, true
	case int:
		return int64(n), true
	}
	return 0, false
}

func _sir_ruby_round(x float64) int64 {
	if x >= 0 {
		return int64(math.Floor(x + 0.5))
	}
	return int64(math.Ceil(x - 0.5))
}

// _sir_is_int reports whether a runtime Value is an integer (not a float).
func _sir_is_int(v Value) bool {
	_, ok := _sir_int_val(v)
	return ok
}

// _sir_floor_div is Ruby's integer division: the quotient FLOORED toward −∞
// (`-7 / 2 == -4`), unlike Go's truncating `/`.  Callers guarantee `b != 0`.
func _sir_floor_div(a, b int64) int64 {
	q := a / b
	if (a%b != 0) && ((a < 0) != (b < 0)) {
		q--
	}
	return q
}

// _sir_pow10 returns 10**n for a small non-negative n.  Callers bound n ≤ 18
// (int64 holds ≤ ~9.2e18), so the result never overflows int64.
func _sir_pow10(n int64) int64 {
	result := int64(1)
	for i := int64(0); i < n; i++ {
		result *= 10
	}
	return result
}

// _sir_round_int_to_multiple rounds `v` to the nearest multiple of `factor`
// half-AWAY-from-zero using all-integer arithmetic (`Integer#round(-n)` /
// `Float#round(<=0)` parity).  `factor >= 1`.  Ruby's result is a bignum that
// may not fit int64; rather than return a two's-complement-wrapped (sign-
// flipped) garbage value, we DEGRADE to the un-rounded receiver when the
// rounded multiple would overflow int64 (the closest representable answer),
// holding the never-surprise floor.  `math.MinInt64` cannot be negated, so it
// takes the same degrade path.
func _sir_round_int_to_multiple(v, factor int64) int64 {
	if v == math.MinInt64 {
		return v
	}
	neg := v < 0
	if neg {
		v = -v
	}
	q := v / factor
	rem := v - q*factor
	if rem*2 >= factor {
		q++
	}
	// Guard `q*factor` against int64 overflow (q, factor both non-negative).
	if factor != 0 && q > math.MaxInt64/factor {
		if neg {
			return -v
		}
		return v
	}
	magnitude := q * factor
	if neg {
		return -magnitude
	}
	return magnitude
}

func _sir_gcd(a, b int64) int64 {
	if a < 0 {
		a = -a
	}
	if b < 0 {
		b = -b
	}
	for b != 0 {
		a, b = b, a%b
	}
	return a
}

// The upper bound on a `pow` result's bit-length before we refuse it.
// Mirrors the Python/TS `_MAX_POW_BITS` (1 << 20): a translated program asking
// for an astronomically large integer power gets a controlled 0 rather than an
// unbounded multiply loop.  int64 can only ever HOLD 63 significant bits, so
// this really guards the LOOP COUNT (the exponent), not the result width.
const _sir_max_pow_bits = 1 << 20

func _sir_int_pow(base, exp int64) int64 {
	if exp < 0 {
		// Only ±1 have integer reciprocals; everything else collapses to 0.
		switch base {
		case 1:
			return 1
		case -1:
			if exp%2 == 0 {
				return 1
			}
			return -1
		}
		return 0
	}
	// Closed-form fast paths for base ∈ {0, 1, -1}: these are O(1) regardless
	// of exponent.  This ALSO closes a DoS gap — the old code exempted them
	// from the `exp > _sir_max_pow_bits` guard but still ran the `exp`-length
	// loop, so `1 ** (1<<40)` spun ~10^12 trivial iterations.
	switch base {
	case 0:
		if exp == 0 {
			return 1 // 0**0 == 1, matching Ruby
		}
		return 0
	case 1:
		return 1
	case -1:
		if exp%2 == 0 {
			return 1
		}
		return -1
	}
	// Refuse an exponent so large the multiply loop would never finish (the
	// int64 result is meaningless past overflow anyway).
	if exp > _sir_max_pow_bits {
		return 0
	}
	var acc int64 = 1
	for i := int64(0); i < exp; i++ {
		acc *= base // int64 wraparound, matching `_sir_times`
	}
	return acc
}

func _sir_digits(n int64) *Seq {
	if n < 0 {
		n = -n
	}
	if n == 0 {
		return &Seq{Items: []Value{int64(0)}}
	}
	out := []Value{}
	for n > 0 {
		out = append(out, n%10)
		n /= 10
	}
	return &Seq{Items: out}
}

// `to_i`-style truncation that also accepts a float receiver (Ruby's
// `3.7.to_i == 3`, `even?`/`odd?` truncate first).  A non-finite float
// degrades to 0 rather than panicking (never-raise floor).
// Lenient float coercion for the numeric OO surface: a non-numeric
// receiver/argument degrades to 0.0 instead of panicking, upholding the
// never-raise-on-the-dispatch-surface invariant (mirrors _sir_as_int_trunc).
func _sir_as_float_lenient(v Value) float64 {
	switch n := v.(type) {
	case int64:
		return float64(n)
	case int:
		return float64(n)
	case float64:
		return n
	}
	return 0
}

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

// ── SIR17 exceptions (E3): panic / recover ─────────────────────
//
// Go has NO native try/catch — it models unwinding with `panic` +
// deferred `recover`.  The emitter maps a SIR `TryCatch` onto an
// immediately-invoked func whose deferred closure calls `recover()`
// and dispatches to the matching rescue clause (see emit.rs).  These
// helpers are the non-native pieces that dispatch needs:
//
//   * `SirError`     — the thrown value: a Ruby/SIR class NAME tag plus
//                      an optional message.  A `raise Foo, "m"` boxes one
//                      of these and `panic`s it.
//   * `_sir_new_error` — construct a `*SirError` for `raise`.
//   * `_sir_exc_value` — the `Value` a `rescue … => e` binds: the caught
//                        `*SirError` boxed back into a `Value` (or, for a
//                        non-`SirError` panic — e.g. a runtime "division by
//                        zero" — a synthesised `StandardError` so ordinary
//                        `rescue`s can still bind something meaningful).
//   * `_sir_rescue_matches` — does the caught value match a clause naming
//                        `classNames`?  This is the ordered, ancestry-aware
//                        type test Ruby's typed `rescue` performs.
//
// SECURITY — the ancestry walk is an EXPLICIT string-map lookup; it never
// reflects on a Go type name.  User-defined class edges are added ONLY via
// `_sir_register_ancestry` (emitted from `ClassDef{superclass:Some}` pairs),
// so a rescue can never resolve to arbitrary host behaviour.  Every walk
// carries a `seen` set so a malicious cyclic hierarchy (`class A<B; class
// B<A`) terminates instead of looping forever.

// A SIR exception: a class-name tag plus an optional message value.  Boxed
// as a `*SirError` and thrown with `panic`.  `Msg` is nil when `raise Foo`
// gave no message (Ruby's default `exception.message` is then the class
// name — see `_sir_format_d`'s SirError arm).
type SirError struct {
	Class string
	Msg   Value
}

// Implement Go's `error` interface so an UNCAUGHT `panic(*SirError)` (a
// raise/runtime-error that no `rescue` matched) prints a readable
// `panic: <Class>: <message>` line — mirroring Ruby's uncaught-exception
// banner — rather than Go's default `(*main.SirError) 0x…` pointer dump.
// Purely cosmetic for the panic path; `recover`/rescue matching still keys
// off the `Class` tag via `_sir_class_of_thrown`, never this string.
func (e *SirError) Error() string {
	if e.Msg == nil {
		return e.Class
	}
	return e.Class + ": " + _sir_format(e.Msg)
}

// Built-in Ruby exception ancestry: subclass name → immediate superclass
// name.  A curated slice of Ruby's tree (the classes a frontend is likely
// to name), each chaining up to `StandardError → Exception`.  Mirrors the
// TS/Python `sir-runtime-exceptions` ANCESTRY table for cross-backend
// parity.  Seeded once at package init; user edges are appended by
// `_sir_register_ancestry`.
//
//	Exception
//	└─ StandardError
//	   ├─ RuntimeError ├─ ArgumentError ├─ TypeError
//	   ├─ NameError ─ NoMethodError      ├─ RangeError
//	   ├─ IndexError ─ KeyError          ├─ ZeroDivisionError
//	   ├─ IOError    ├─ StopIteration    └─ NotImplementedError
var _sir_ancestry = map[string]string{
	"RuntimeError":        "StandardError",
	"ArgumentError":       "StandardError",
	"TypeError":           "StandardError",
	"NameError":           "StandardError",
	"NoMethodError":       "NameError",
	"IndexError":          "StandardError",
	"KeyError":            "IndexError",
	"RangeError":          "StandardError",
	"ZeroDivisionError":   "StandardError",
	"IOError":             "StandardError",
	"StopIteration":       "StandardError",
	"NotImplementedError": "StandardError",
	"StandardError":       "Exception",
}

// Register user-defined `subclass → superclass` edges (from
// `ClassDef{superclass:Some}`).  Called once at program init.  A built-in
// edge is never overwritten (the built-in table is authoritative for
// built-in names); a user redefinition of a built-in name is ignored so
// the curated hierarchy stays intact.
func _sir_register_ancestry(edges map[string]string) {
	for sub, sup := range edges {
		if _, isBuiltin := _sir_ancestry[sub]; !isBuiltin {
			_sir_ancestry[sub] = sup
		}
	}
}

// Construct a SIR exception for `raise Class, msg`.  `msg` may be nil
// (bare `raise Class`), in which case the class name serves as the
// message (Ruby's default).
func _sir_new_error(class string, msg Value) *SirError {
	return &SirError{Class: class, Msg: msg}
}

// The `Value` a `rescue => e` binds for a recovered panic `r`.
//
//   - A `*SirError` (from `raise`) is boxed straight back as the binding.
//   - Any OTHER recovered value (a runtime panic string like "division by
//     zero", or a stray `panic(x)`) is wrapped as a `StandardError` whose
//     message is the value's printed form — so `rescue StandardError => e`
//     still catches Go-level runtime failures and `e` is meaningful.
func _sir_exc_value(r any) Value {
	if se, ok := r.(*SirError); ok {
		return se
	}
	return &SirError{Class: "StandardError", Msg: _sir_format(r)}
}

// The SIR class name of a recovered panic value.  A `*SirError` reports
// its tag; anything else is treated as `StandardError` — the everyday
// rescuable root — so a native Go runtime panic is catchable by an
// ordinary `rescue`.
func _sir_class_of_thrown(r any) string {
	if se, ok := r.(*SirError); ok {
		return se.Class
	}
	return "StandardError"
}

// True iff `actual` is `target` or descends from it via `_sir_ancestry`.
// The `seen` set makes the walk total even for a cyclic user hierarchy.
func _sir_is_ancestor_or_self(actual string, target string) bool {
	cur := actual
	seen := make(map[string]bool)
	for cur != "" && !seen[cur] {
		if cur == target {
			return true
		}
		seen[cur] = true
		cur = _sir_ancestry[cur] // "" when `cur` has no registered super
	}
	return false
}

// Does a recovered panic `r` match a rescue clause naming `classNames`?
//
//   - EMPTY `classNames` is a bare `rescue` (catch-all) ⇒ always true.
//   - `Exception` is Ruby's universal root ⇒ matches anything.
//   - Otherwise `r`'s class must equal, or descend from, some named class
//     (per `_sir_ancestry`; user classes match by exact name or via
//     registered edges).
//
// The emitted deferred recover calls this once per clause, in SOURCE
// order, running the first match and re-`panic`king if none match.
func _sir_rescue_matches(r any, classNames []string) bool {
	if len(classNames) == 0 {
		return true
	}
	actual := _sir_class_of_thrown(r)
	for _, name := range classNames {
		if name == "Exception" || _sir_is_ancestor_or_self(actual, name) {
			return true
		}
	}
	return false
}

// ── O4 user-defined-class OOP (instances, method tables, self/super) ──
//
// The Ruby→SIR frontend HOISTS every `def` inside a class to a detached
// top-level function with NO receiver — nothing in the IR records that
// `speak` belongs to `Dog`.  We recover that association at RUNTIME with two
// explicit method tables, populated by emitted `__def_method__` /
// `__def_class_method__` registrations.  This is the Go analogue of the
// Python/TS `sir-runtime-oop` `call_new`/`call_super`/`call_method`
// user-object path, ported for cross-backend behavioural parity.
//
// ── SECURITY (the C3 RCE lesson, restated for OOP) ────────────────────
// Dispatch is ONLY an explicit map lookup on the `(class, method)` key —
// NEVER Go `reflect`/`MethodByName` on a source-derived name.  A class or
// method perversely named `constructor` / `__proto__` / `initialize` is JUST
// a map key: absent from the table ⇒ a clean miss ⇒ the ordinary
// `_sir_method_unknown` (NoMethodError-shaped) floor or a `nil` `super`
// result, never host behaviour.  Every ancestry walk carries a `seen` set so
// a malicious cyclic hierarchy (`class A<B; class B<A`) TERMINATES instead of
// looping forever.  Self-stack pops go through `defer`, so even a panic inside
// a method body still unwinds the stack correctly.

// A SIR object instance: a class-name tag plus a bag of instance variables.
// `Ivars` is keyed by the FULL Ruby sigil name (`"@name"`), matching how the
// frontend lowers an `@ivar` reference — the sigil is part of the key, never
// stripped, so `@x` and a hypothetical local `x` can never collide.
type SirInstance struct {
	Class string
	Ivars map[string]Value
}

// Allocate a fresh instance tagged with `cls` and an empty ivar bag.
func _sir_new_instance(cls string) *SirInstance {
	return &SirInstance{Class: cls, Ivars: make(map[string]Value)}
}

// ── Method tables ─────────────────────────────────────────────────────
//
// Instance methods (`def m`) and class methods (`def self.m`) live in two
// separate maps keyed by `class + "\x00" + method`.  A NUL joiner is used
// (rather than a `[2]string` composite key) because a NUL byte cannot appear
// in a Ruby class or method identifier, so the flattened key is unambiguous —
// and, crucially, it is a PLAIN VALUE lookup with no reflection.  The value is
// a `*Closure` (the hoisted top-level function captured by an emitted
// `MakeClosure`), invoked via `_sir_apply`.
var _sir_instance_methods = make(map[string]Value)
var _sir_class_methods = make(map[string]Value)

// The NUL-joined table key for a `(class, method)` pair.
func _sir_method_key(cls string, method string) string {
	return cls + "\x00" + method
}

// Register an instance method (`__def_method__`) / class method
// (`__def_class_method__`).  Called once per method at program init.  Each
// returns the closure so the emitter may use the registration in expression
// position, mirroring the other builtins' `Value`-returning convention.
func _sir_def_method(cls string, method string, fn Value) Value {
	_sir_instance_methods[_sir_method_key(cls, method)] = fn
	return fn
}

func _sir_def_class_method(cls string, method string, fn Value) Value {
	_sir_class_methods[_sir_method_key(cls, method)] = fn
	return fn
}

// ── MX5 mixins: per-owner included-module list ────────────────────────
//
// `include M` inside `class C` (or `module C`) records that `C`'s method
// resolution must consult `M` — BEFORE ascending to `C`'s superclass, and
// AFTER `C`'s own methods.  We keep this as an explicit per-owner slice of
// module names, appended in SOURCE (include) order.  Ruby searches the
// MOST-RECENTLY-included module first, so the resolution walk iterates this
// slice in REVERSE (see `_sir_resolve_instance_method`).
//
// A module is itself an "owner" in the method table (`module M; def foo`
// registers `_sir_instance_methods[("M","foo")]` via `__def_method__`), so a
// module that itself `include`s another module contributes ITS includes too
// when the walk recurses into it — Ruby's transitive mixin inclusion.
//
// SECURITY: this is a plain `map[string][]string` keyed by source-derived
// NAMES with no reflection (the C3 RCE discipline).  The MRO walk carries a
// `seen` set so a module that (transitively) includes itself TERMINATES.
var _sir_included_modules = make(map[string][]string)

// `__include__("Owner", "M")` — record that `Owner` mixes in `M`.  Appends in
// include order (idempotent duplicates are harmless: the MRO walk's `seen` set
// de-dups a diamond, and appending a name twice just makes the second visit a
// no-op).  Returns nil (the directive has no Ruby value the emitter needs).
func _sir_include(owner string, module string) Value {
	_sir_included_modules[owner] = append(_sir_included_modules[owner], module)
	return nil
}

// `__extend__("Owner", "M")` — mix `M`'s INSTANCE methods in as `Owner`'s
// CLASS (singleton) methods, so they become callable as `Owner.method`.  We
// SNAPSHOT `M`'s registered instance methods (including those `M` itself
// includes, via the same MRO walk used for instances) and copy each into
// `Owner`'s class-method table.  An entry `Owner` already defines is NOT
// overwritten (a class/own method shadows an extended module method), matching
// Ruby's singleton-first precedence.  Copy-at-extend-time is the v0 model:
// methods defined on `M` AFTER the `extend` are not retroactively added, which
// is sufficient because the frontend emits every `__def_method__` for `M`
// before any `__extend__` that names it (registrations run in source order,
// module def before the including class).
func _sir_extend(owner string, module string) Value {
	for _, name := range _sir_module_method_names(module) {
		key := _sir_method_key(owner, name)
		if _, exists := _sir_class_methods[key]; exists {
			continue
		}
		if fn, ok := _sir_resolve_instance_method(module, name); ok {
			_sir_class_methods[key] = fn
		}
	}
	return nil
}

// The instance-method NAMES reachable on `module` (its own defs plus those of
// modules IT includes), for `_sir_extend` to copy.  Walks the same
// include-list MRO as instance resolution, `seen`-guarded against a cyclic
// include, and de-dups names so each method is copied once (the earliest,
// most-specific definition wins — we only add a name the first time it is
// seen).
func _sir_module_method_names(module string) []string {
	var names []string
	added := make(map[string]bool)
	seenOwners := make(map[string]bool)
	var walk func(owner string)
	walk = func(owner string) {
		if owner == "" || seenOwners[owner] {
			return
		}
		seenOwners[owner] = true
		prefix := owner + "\x00"
		for key := range _sir_instance_methods {
			if len(key) > len(prefix) && key[:len(prefix)] == prefix {
				name := key[len(prefix):]
				if !added[name] {
					added[name] = true
					names = append(names, name)
				}
			}
		}
		mods := _sir_included_modules[owner]
		for i := len(mods) - 1; i >= 0; i-- {
			walk(mods[i])
		}
	}
	walk(module)
	return names
}

// Resolve `method` on `cls` following Ruby's MRO (Method Resolution Order):
//
//	cls  →  cls's included modules (REVERSE / most-recent-first)  →
//	cls's superclass  →  its included modules  →  …  →  Object
//
// A class's OWN method shadows any module it includes; a module method shadows
// the superclass's (module comes before the superclass in the ancestor list).
// A module included via TWO paths (a diamond) resolves ONCE, at its earliest
// position, because the `seen` set skips an owner already visited.
//
// The walk is a depth-first, most-recent-first, de-duplicated linearisation
// (the exact order the spec's truth table documents).  It shares the runtime's
// single `_sir_ancestry` table for the superclass chain (the same table
// exception `rescue` uses).  The `seen` set makes the walk TOTAL even for a
// cyclic class hierarchy OR a self-including module.  Returns the closure and
// `true` on a hit, or `(nil, false)` when unresolved.
func _sir_resolve_instance_method(cls string, method string) (Value, bool) {
	seen := make(map[string]bool)
	// `resolveOwner` checks `owner`'s own methods, then (reverse-order) its
	// included modules — each of which may itself include further modules,
	// so it recurses.  Returns the closure on the first hit.
	var resolveOwner func(owner string) (Value, bool)
	resolveOwner = func(owner string) (Value, bool) {
		if owner == "" || seen[owner] {
			return nil, false
		}
		seen[owner] = true
		if fn, ok := _sir_instance_methods[_sir_method_key(owner, method)]; ok {
			return fn, true
		}
		// Included modules search most-recently-included first (Ruby's rule),
		// so iterate the include-order slice in REVERSE.  A module search
		// recurses so a module that itself includes another module is honoured.
		mods := _sir_included_modules[owner]
		for i := len(mods) - 1; i >= 0; i-- {
			if fn, ok := resolveOwner(mods[i]); ok {
				return fn, true
			}
		}
		return nil, false
	}
	cur := cls
	for cur != "" {
		if seen[cur] {
			// The class chain itself is cyclic (`A<B, B<A`): stop.
			break
		}
		if fn, ok := resolveOwner(cur); ok {
			return fn, true
		}
		cur = _sir_ancestry[cur] // "" when `cur` has no registered super
	}
	return nil, false
}

// `Foo.bar(args…)` — a CLASS-method call (`__class_method__`).  Resolves `bar`
// in `Foo`'s class-method table, walking the ancestry so an inherited
// `def self.bar` is found, and INCLUDING methods mixed in via `extend` (which
// `_sir_extend` copied into the class-method table).  No `self` is pushed —
// v0 class methods run without an instance receiver.  An unresolved name hits
// the controlled NoMethodError floor, never reflection.
func _sir_call_class_method(cls string, method string, args ...Value) Value {
	cur := cls
	seen := make(map[string]bool)
	for cur != "" && !seen[cur] {
		if fn, ok := _sir_class_methods[_sir_method_key(cur, method)]; ok {
			return _sir_apply(fn, args)
		}
		seen[cur] = true
		cur = _sir_ancestry[cur]
	}
	panic(_sir_new_error("NoMethodError",
		Value("undefined method '"+method+"' for "+cls)))
}

// ── Current-self stack + instance-variable store ──────────────────────
//
// The single-threaded self-stack: `_sir_call_new` / instance-method dispatch
// push the receiver before running a body and pop after (via `defer`), so an
// `@ivar` reference inside the body reads the right object with NO explicit
// `self` parameter.  This is the documented v0 model — correct for the
// single-threaded transpiled scripts we target; true per-object/per-thread
// binding is out of scope for v0 (consistent with the runtime note).
var _sir_self_stack []*SirInstance

// A program that never pushes a self (a top-level `@x`) still needs somewhere
// to put instance variables; this default object provides it so `@x`
// reads/writes never panic.
var _sir_default_self = &SirInstance{Class: "Object", Ivars: make(map[string]Value)}

func _sir_current_self_obj() *SirInstance {
	if len(_sir_self_stack) > 0 {
		return _sir_self_stack[len(_sir_self_stack)-1]
	}
	return _sir_default_self
}

func _sir_push_self(obj *SirInstance) {
	_sir_self_stack = append(_sir_self_stack, obj)
}

func _sir_pop_self() {
	if len(_sir_self_stack) > 0 {
		_sir_self_stack = _sir_self_stack[:len(_sir_self_stack)-1]
	}
}

// `__self__` — a bare `self` in a method body.  Returns the current receiver
// (top of the self-stack), or `nil` (Ruby `nil`) at top level where no
// receiver is bound — never the internal default-self sentinel.
func _sir_current_self() Value {
	if len(_sir_self_stack) > 0 {
		return _sir_self_stack[len(_sir_self_stack)-1]
	}
	return nil
}

// `@ivar` read: the value on the current self, or `nil` for an unset ivar
// (Ruby reads an unset `@x` as nil — no error).
func _sir_ivar_get(name string) Value {
	if v, ok := _sir_current_self_obj().Ivars[name]; ok {
		return v
	}
	return nil
}

// `@ivar` write on the current self; returns the written value so `@x = v`
// can be used in expression position.
func _sir_ivar_set(name string, value Value) Value {
	_sir_current_self_obj().Ivars[name] = value
	return value
}

// ── Class-variable store (`@@cvar`) ───────────────────────────────────
//
// Ruby class variables are shared across a class and its instances.  The v0
// model keys them by their bare `@@name` in a single flat namespace (matching
// the Python/TS reference's single-namespace `cvar` store), which faithfully
// models single-class programs; per-class-hierarchy scoping awaits a frontend
// that threads the enclosing class.
var _sir_cvars = make(map[string]Value)

func _sir_cvar_get(name string) Value {
	if v, ok := _sir_cvars[name]; ok {
		return v
	}
	return nil
}

func _sir_cvar_set(name string, value Value) Value {
	_sir_cvars[name] = value
	return value
}

// `Foo.new(args…)` — allocate a `cls` instance and run its `initialize`.
//
// Allocates via `_sir_new_instance`, pushes the new object as the current
// self, and — if an `initialize` is registered for `cls` or any ancestor —
// invokes it with `args` (so `@ivar` assignments in the constructor land on
// the new object).  Self is popped via `defer` (so a panic in `initialize`
// still unwinds cleanly), and the object is always returned — even with no
// `initialize` (a plain allocation).
func _sir_call_new(cls string, args ...Value) Value {
	obj := _sir_new_instance(cls)
	_sir_push_self(obj)
	defer _sir_pop_self()
	if initializer, ok := _sir_resolve_instance_method(cls, "initialize"); ok {
		_sir_apply(initializer, args)
	}
	return obj
}

// `super` — re-run `method` from `cls`'s PARENT.
//
// Walks from `_sir_ancestry[cls]` upward and invokes the first ancestor
// implementation of `method` with `args`, keeping the CURRENT self bound
// (`super` is a re-dispatch on the same receiver, so there is no push/pop
// here).  If no ancestor defines the method, returns `nil` (Ruby `nil`) — the
// runtime's honest floor, consistent with `_sir_call_method`'s user-object
// path.
func _sir_call_super(method string, cls string, args ...Value) Value {
	parent := _sir_ancestry[cls] // "" when `cls` has no registered super
	if parent == "" {
		return nil
	}
	if fn, ok := _sir_resolve_instance_method(parent, method); ok {
		return _sir_apply(fn, args)
	}
	return nil
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
            "_sir_print", "_sir_puts", "_sir_global_set", "_sir_global_get",
            "_sir_apply", "_sir_make_closure", "_sir_intern", "_sir_truthy",
            "_sir_format", "_sir_builtin_closure", "_sir_call_builtin_by_name",
            // E3 exception helpers.
            "_sir_new_error", "_sir_exc_value", "_sir_rescue_matches",
            "_sir_register_ancestry",
        ] {
            assert!(RUNTIME.contains(s), "missing: {}", s);
        }
    }

    // E3: the ancestry table and rescue matcher must be present with the
    // built-in edges the TS/Python reference bakes in.
    #[test]
    fn runtime_includes_exception_ancestry() {
        assert!(RUNTIME.contains("type SirError struct"));
        assert!(RUNTIME.contains("var _sir_ancestry = map[string]string{"));
        assert!(RUNTIME.contains(r#""StandardError":       "Exception""#));
        assert!(RUNTIME.contains(r#""NoMethodError":       "NameError""#));
        assert!(RUNTIME.contains(r#""KeyError":            "IndexError""#));
        // The cycle-guard `seen` set must be present (no unbounded walk).
        assert!(RUNTIME.contains("seen := make(map[string]bool)"));
    }

    // MX5: the mixin tables + helpers must be present, and the MRO walk must
    // consult the per-owner included-module list (reverse order).
    #[test]
    fn runtime_includes_mixin_helpers() {
        assert!(RUNTIME.contains("var _sir_included_modules = make(map[string][]string)"));
        assert!(RUNTIME.contains("func _sir_include(owner string, module string) Value"));
        assert!(RUNTIME.contains("func _sir_extend(owner string, module string) Value"));
        assert!(RUNTIME.contains(
            "func _sir_call_class_method(cls string, method string, args ...Value) Value"
        ));
        // The MRO walk consults the included-module list in REVERSE (most
        // recently included first) and recurses per owner.
        assert!(RUNTIME.contains("mods := _sir_included_modules[owner]"));
        assert!(RUNTIME.contains("for i := len(mods) - 1; i >= 0; i--"));
    }
}
