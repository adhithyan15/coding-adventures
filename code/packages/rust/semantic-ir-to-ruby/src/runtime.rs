//! Inlined Ruby runtime — a small preamble pasted into every emitted artifact.
//!
//! The Ruby backend produces **self-contained** output: every generated `.rb`
//! file embeds the handful of helpers it needs, so `ruby <file>.rb` runs with
//! no gems.  This mirrors the Go/Rust/C backends (inlined runtime) rather than
//! the Python/TypeScript `sir-runtime-*` import model.
//!
//! Ruby's semantics already match the SIR's, so the runtime is tiny: only a
//! cons-`Pair` shim, a global store, a display path that honours the
//! [display convention](../../../specs/sir-display-convention.md), and a
//! builtin-as-value dispatcher.  Truthiness, arithmetic, equality, symbols, and
//! closures are all native Ruby.
//!
//! ## Display convention
//!
//! The single placeholder `__SIR_DISPLAY_RUBY__` is substituted by the emitter
//! with `true` (Ruby-sourced module → booleans render as `true`/`false`, `nil`
//! as the empty string) or `false` (the default Lisp rendering — `#t`/`#f`,
//! `nil`).  The substitution is a **boolean-selected literal**, never
//! source-derived text, so it can never inject into the emitted Ruby.

/// The Ruby runtime, as a single string constant.  `emit::emit_module` prepends
/// it to every artifact, first replacing the `__SIR_DISPLAY_RUBY__` placeholder.
pub const RUNTIME: &str = r####"# ── inlined SIR runtime (semantic-ir-to-ruby) ──
# Ruby's semantics match the SIR's, so this preamble is small.

SIR_DISPLAY_RUBY = __SIR_DISPLAY_RUBY__

# A cons cell — the one SIR primitive with no native Ruby type.
SirPair = Struct.new(:car, :cdr)

# Name-keyed global store (the _init function writes it via global_set).
$sir_globals = {}

# SIR truthiness: only nil and false are falsy — which is exactly Ruby's own
# `if` test, so this is a pass-through kept for uniformity with the other
# backends and for values produced by the runtime.
def sir_truthy(v)
  !v.nil? && v != false
end

def sir_car(v) = v.is_a?(SirPair) ? v.car : nil
def sir_cdr(v) = v.is_a?(SirPair) ? v.cdr : nil
def sir_cons(a, b) = SirPair.new(a, b)

def sir_is_null(v)   = v.nil?
def sir_is_pair(v)   = v.is_a?(SirPair)
def sir_is_number(v) = v.is_a?(Numeric)
def sir_is_symbol(v) = v.is_a?(Symbol)

# Structural, symbol-aware equality.  Ruby `==` already compares Integers,
# Floats, Strings, Symbols, true/false/nil, and (recursively, by value) Structs
# such as SirPair — so it is exactly the SIR `=` semantics.
def sir_eq(a, b) = a == b

# Call a closure value (a Ruby lambda) with the given arguments.
def sir_apply(target, *args) = target.call(*args)

# OOP slice 1 — `Foo.new(args…)` construction (the frontend's `__new__`).
# A `def initialize` is registered like every method (slice 2) under the reserved
# `sir_um_` prefix as `sir_um_initialize` — NOT Ruby's native `initialize`, which
# `Class#new` would call.  So `.new` alone would allocate an object whose
# constructor never runs (its `@ivar` initialisers skipped, leaving them nil).
# Mirror the Go/C/Rust runtimes explicitly: `allocate` a bare instance (skipping
# the empty native `initialize`), and — if the class or any ancestor defines
# `sir_um_initialize` — invoke it on the new object with `args`, so the
# constructor's `@ivar` assignments land on it.  Dispatch stays CLOSED: the method
# name is the fixed literal `sir_um_initialize`, never source-derived, so no
# reflection/eval sink is reachable (the repo's anti-RCE discipline, as with
# `public_send` dispatch).  Always returns the object, even with no constructor
# (a plain allocation) — matching `Class#new` on a class with no `initialize`.
def sir_new(cls, *args)
  obj = cls.allocate
  obj.public_send(:sir_um_initialize, *args) if obj.respond_to?(:sir_um_initialize)
  obj
end

# OOP slice 6 — the class that owns a `@@class variable` in the CURRENT method.
# A method body runs in a hoisted top-level function (not a lexical class scope),
# so `@@x` cannot be written directly ("class variable access from toplevel").
# Instead the emitter routes `@@x` through `.class_variable_get/set` on the owner
# resolved here: inside an INSTANCE method `self` is the receiver, so the owner is
# `self.class`; inside a CLASS method `self` IS the class (a `Module`), so it is
# the owner itself.  This gives ONE class in both contexts, so an instance method
# and a class method share the same `@@x` (matching Ruby).
def sir_cvar_owner(s) = s.is_a?(Module) ? s : s.class

# Indexed write `a[i] = v` with the SIR bounds rule.  The reference
# (`_sir_seq_set`) treats ONLY `0 <= i < length` as valid and RAISES on a
# negative or out-of-range index — whereas Ruby's native `a[i] = v` would
# silently pad with nils (i past the end) or count from the end (negative i).
# We enforce the reference rule so every backend agrees, and return the value
# (an indexed assignment evaluates to its right-hand side).
def sir_seq_set(a, i, v)
  raise "sequence index out of range: #{i}" if i < 0 || i >= a.length
  a[i] = v
  v
end

# ── SIR26 integer conversions ──
# Reduce an Integer to a fixed width by two's-complement reinterpretation — the
# rendering of an Expr::Convert.  Ruby's Integer is arbitrary precision and its
# bitwise ops use an (infinite) two's-complement model, so `v & mask` is exact
# even when v is negative (e.g. sir_u8(-1) == 255).  Unsigned masks; signed
# masks then folds the sign bit.  (A target width of Arbitrary is the identity
# and is emitted with no helper.)
def sir_u8(v)   = v & 0xFF
def sir_u16(v)  = v & 0xFFFF
def sir_u32(v)  = v & 0xFFFFFFFF
def sir_u64(v)  = v & 0xFFFFFFFFFFFFFFFF
def sir_u128(v) = v & ((1 << 128) - 1)
def sir_i8(v)
  m = v & 0xFF
  m >= 0x80 ? m - 0x100 : m
end
def sir_i16(v)
  m = v & 0xFFFF
  m >= 0x8000 ? m - 0x10000 : m
end
def sir_i32(v)
  m = v & 0xFFFFFFFF
  m >= 0x80000000 ? m - 0x100000000 : m
end
def sir_i64(v)
  m = v & 0xFFFFFFFFFFFFFFFF
  m >= 0x8000000000000000 ? m - 0x10000000000000000 : m
end
def sir_i128(v)
  m = v & ((1 << 128) - 1)
  m >= (1 << 127) ? m - (1 << 128) : m
end

# C truncating division / remainder (SIR27 `tdiv`/`tmod`).  Ruby's Integer#/ and
# #% FLOOR (toward -inf), but C TRUNCATES toward zero, so `-7 / 2` must be -3 not
# -4.  Integer#remainder already gives C's remainder (sign of the dividend), and
# `(a - a.remainder(b))` is an exact multiple of b, so flooring it recovers the
# truncated quotient.  Division by zero raises (as in C it is undefined).
def sir_tdiv(a, b) = (a - a.remainder(b)) / b
def sir_tmod(a, b) = a.remainder(b)

# SIR21 T3b-2 `div_true` — ALWAYS true-divides, even for two Integer
# operands (`sir_true_div(6, 3) == 2.0`, not `2`).  Ruby's native `/`
# can't be reused directly for two reasons: `Integer#/` floors instead of
# true-dividing, and `Float#/0` silently returns `Infinity` (Ruby does
# NOT raise there, unlike `Integer#/0`) — so a naive `a.to_f / b` would
# let a zero divisor through as IEEE `Infinity`/`NaN` rather than the
# typed `ZeroDivisionError` every sibling division op raises.  The
# explicit check below closes that gap before the float divide ever runs.
def sir_true_div(a, b)
  raise ZeroDivisionError, "divided by 0" if b == 0
  a.to_f / b.to_f
end

# The key is normalised with to_s so a name that arrives as a Symbol (how the
# _init function's global_set passes it) and the same name as a String (how a
# VarRef Global reads it) hit the same entry.
def sir_global_get(name) = $sir_globals[name.to_s]
def sir_global_set(name, val)
  $sir_globals[name.to_s] = val
  val
end

# ── display ──
def sir_fmt(v)
  case v
  when nil        then SIR_DISPLAY_RUBY ? "" : "nil"
  when true       then SIR_DISPLAY_RUBY ? "true"  : "#t"
  when false      then SIR_DISPLAY_RUBY ? "false" : "#f"
  when SirPair    then sir_fmt_pair(v)
  when Symbol     then v.to_s
  when Float      then sir_fmt_float(v)
  when SirSymTerm then sir_sym_to_s(v)
  else v.to_s
  end
end

def sir_fmt_pair(v)
  out = +"("
  cur = v
  first = true
  loop do
    if cur.is_a?(SirPair)
      out << " " unless first
      first = false
      out << sir_fmt(cur.car)
      cur = cur.cdr
    elsif cur.nil?
      break
    else
      out << " . " << sir_fmt(cur)
      break
    end
  end
  out << ")"
end

# Ruby prints an integral Float as "3.0"; keep that, and render non-finite
# values the Ruby way (Infinity / NaN).
def sir_fmt_float(f)
  return "NaN" if f.nan?
  return (f.positive? ? "Infinity" : "-Infinity") if f.infinite?
  f.to_s
end

# C-printf-faithful float formatting for the `fmt_float` builtin (SIR27
# milestone 10): render `value` as C's `printf` would for the given conversion
# `kind` ('f'/'F'/'e'/'E'/'g'/'G') and `precision`.  Ruby's `sprintf` is
# C-compatible, and we switch on the fixed kind character (never interpolating
# a source-derived format string), so `printf("%.2f", 3.14159)` and the emitted
# C produce byte-identical "3.14".
def sir_fmt_float_c(value, precision, kind)
  v = value.to_f
  p = precision.to_i
  case kind
  when "f" then sprintf("%.*f", p, v)
  when "F" then sprintf("%.*F", p, v)
  when "e" then sprintf("%.*e", p, v)
  when "E" then sprintf("%.*E", p, v)
  when "g" then sprintf("%.*g", p, v)
  when "G" then sprintf("%.*G", p, v)
  else sprintf("%.*f", p, v)
  end
end

# `per_value`'s ARRAY-UNPACKING rule (`unpack_arrays: true`, `puts`'s
# behavior) -- distinct from `sir_fmt`'s general case (`v.to_s`, which
# bracket-displays an Array, e.g. `"[1, 2, 3]"`). Real Ruby's `Kernel#puts`
# special-cases an Array argument: each element gets its OWN line,
# RECURSIVELY flattening nested arrays, and an EMPTY array prints nothing
# at all (not even a blank line) -- `puts [1, [2, 3], 4]` ->
# "1\n2\n3\n4\n"; `puts []` -> (nothing). A Hash argument is NOT unpacked
# (only Array), so this checks `Array` specifically. No depth cap is
# needed here (unlike the C backend's `_sir_puts_one`): a self-referential
# array would raise Ruby's own `SystemStackError` on infinite recursion --
# safe, since this runs under a real Ruby VM with its own stack-overflow
# protection, not raw C recursion.
#
# SIR28 §2.1: `__sys_write__`, the general console-output primitive every
# frontend lowers `print`/`puts`/`console.log`/etc. to. It generalizes what
# used to be several backend-hardcoded newline policies into ONE operation
# parameterized by policy flags carried as DATA (validated by
# `semantic-ir`'s validator against a closed enum, SIR28 §2.2) -- the root
# cause SIR28 exists to fix: real Ruby's `print` never newline-terminates,
# Python's `print()`/JS's `console.log` always do, but before SIR28 all
# three lowered to the identical `BuiltinCall("print", ...)` this backend
# had no way to tell apart.
#
# `stream`: "stdout" | "stderr". `terminator`: "none" (write each value
# back to back, no newline -- matches Ruby's `print`) | "per_value" (one
# newline per value, honouring `unpack_arrays` -- matches Ruby's `puts`) |
# "once" (Python `print`/JS `console.log` -- space-join every value, one
# trailing newline). Unlike the C/Go/Rust backends, no
# compile-time dispatch is needed here -- `stream`/`terminator` arrive as
# ordinary Ruby string arguments (already validated to a closed set by
# `semantic-ir` before this backend ever sees them) and this function
# branches on them directly at Ruby runtime, exactly like every other
# `sir_*` helper in this file.
def sir_write_puts_one(out, x)
  if x.is_a?(Array)
    x.each { |e| sir_write_puts_one(out, e) }
  else
    out.write(sir_fmt(x))
    out.write("\n")
  end
end

def sir_write(stream, terminator, unpack_arrays, *xs)
  out = stream == "stderr" ? STDERR : STDOUT
  case terminator
  when "per_value"
    if xs.empty?
      out.write("\n")
    else
      xs.each do |x|
        if unpack_arrays
          sir_write_puts_one(out, x)
        else
          out.write(sir_fmt(x))
          out.write("\n")
        end
      end
    end
  when "once"
    out.write(xs.map { |x| sir_fmt(x) }.join(" "))
    out.write("\n")
  else
    xs.each { |x| out.write(sir_fmt(x)) }
  end
  nil
end

# A builtin used in value position becomes a lambda that dispatches by name.
# The case IS the allowlist — an unknown name fails cleanly, never resolving
# reflectively (the repo's anti-RCE discipline).
def sir_builtin_dispatch(name, args)
  case name
  when "+" then args.reduce(:+)
  when "-" then args.length == 1 ? -args[0] : args.reduce(:-)
  when "*" then args.reduce(:*)
  when "/" then args.reduce(:/)
  when "=" then sir_eq(args[0], args[1])
  when "<" then args[0] < args[1]
  when ">" then args[0] > args[1]
  when "cons" then sir_cons(args[0], args[1])
  when "car" then sir_car(args[0])
  when "cdr" then sir_cdr(args[0])
  when "null?"   then sir_is_null(args[0])
  when "pair?"   then sir_is_pair(args[0])
  when "number?" then sir_is_number(args[0])
  when "symbol?" then sir_is_symbol(args[0])
  else
    STDERR.puts("sir: undefined builtin '#{name}'")
    exit(1)
  end
end

def sir_builtin_closure(name)
  ->(*args) { sir_builtin_dispatch(name, args) }
end

# ── SIR22 array/matrix domain ──
#
# `ArrayLit`/`Range`/`MatMul`/`ElementwiseOp`/`Transpose`/`IndexGet` (and
# `Stmt::IndexSet`) lower to calls into `sir_array_*` below — an inlined
# port of `semantic-ir-to-javascript`'s own already-proven `ArrayRt`
# sub-runtime (itself a plain-JS port of the published
# `@coding-adventures/sir-runtime-array` package), so the Ruby artifact
# stays self-contained like every other feature batch in this file.
#
# ## Value model
#
# `SirNDArray(shape, data)` — dense, rectangular, COLUMN-MAJOR storage
# (Fortran/MATLAB order), mirroring `array_runtime::value::Array`
# field-for-field. `shape == []` is a scalar, `[n]` a vector (an `n×1`
# column for row/column purposes), `[r, c]` a matrix — this port's whole
# scope, like the JS/TS references, is rank <= 2. Unlike JS (whose
# `Float64Array` forces every element to an IEEE double), `data` here is a
# plain Ruby Array holding whatever Numeric type the source arithmetic
# naturally produces (Integer stays Integer through `+`/`-`/`*`; only
# `Div`/`Pow` force a Float result, matching this crate's own `div_true`
# precedent) — so an all-integer computation like a 2x2 `matmul` prints
# its result without a spurious ".0", matching this backend's existing
# `sir_fmt_float` display convention (which deliberately keeps ".0" on a
# REAL Float, e.g. from `Div`).
#
# The SIR22 "APL addendum" (`Reduce`/`Scan`/`OuterProduct`/`Shape`/
# `Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`, Phase A Slice 3) —
# these nine are implemented below, in the "SIR22 addendum" section further
# down this file — an inlined port of `semantic-ir-to-javascript`'s own
# already-proven addendum functions, following the exact same
# `sir_array_*` naming/style convention as the base cut above.
SirNDArray = Struct.new(:shape, :data)

# SECURITY: every factory below validates a shape/output size *before*
# allocating a Ruby Array from it — a compiled program's array sizes come
# from potentially attacker-influenced runtime values (loop counts, parsed
# input, ...), not fixed compile-time constants, so an unbounded or
# malformed shape must fail cleanly with a catchable exception rather than
# let `Array.new(n)` itself exhaust memory. Mirrors the JS backend's own
# `MAX_ELEMENTS` bound exactly, so behaviour is identical across both.
SIR_ARRAY_MAX_ELEMENTS = 1 << 26 # 67,108,864
SIR_ARRAY_RANGE_EPSILON = 1e-9

# `Number.isInteger`-equivalent: true for a Ruby Integer, or a finite Float
# equal to its own truncation (e.g. `2.0`, not `2.5`/NaN/Infinity).
def sir_array_integer_dim?(d)
  d.is_a?(Integer) || (d.is_a?(Float) && d.finite? && d == d.to_i)
end

# `Number.isFinite`-equivalent over this domain's numeric literals (always
# Integer or Float): every Ruby Integer is finite; a Float is finite only
# when neither NaN nor +/-Infinity.
def sir_array_finite?(x)
  x.is_a?(Integer) || (x.is_a?(Float) && x.finite?)
end

def sir_array_checked_shape_size(shape)
  unless shape.all? { |d| sir_array_integer_dim?(d) && d >= 0 }
    raise "sir_array_checked_shape_size: shape #{shape} has a negative or non-integer dimension"
  end
  n = shape.reduce(1) { |acc, d| acc * d }
  if n > SIR_ARRAY_MAX_ELEMENTS
    raise "sir_array_checked_shape_size: shape #{shape} (#{n} elements) exceeds the #{SIR_ARRAY_MAX_ELEMENTS}-element cap"
  end
  n
end

def sir_array_ndarray(shape, data)
  raise "sir_array_ndarray: data must be an Array" unless data.is_a?(Array)
  n = sir_array_checked_shape_size(shape)
  if n != data.length
    raise "sir_array_ndarray: shape #{shape} implies #{n} elements, got #{data.length}"
  end
  SirNDArray.new(shape, data)
end

def sir_array_from_rows(rows)
  nrows_in = rows.length
  return sir_array_ndarray([0, 0], []) if nrows_in == 0
  ncols_in = rows[0].length
  raise "sir_array_from_rows: ragged rows" if rows.any? { |r| r.length != ncols_in }
  n = sir_array_checked_shape_size([nrows_in, ncols_in])
  data = Array.new(n)
  (0...nrows_in).each do |r|
    (0...ncols_in).each do |c|
      data[c * nrows_in + r] = rows[r][c] # column-major store
    end
  end
  sir_array_ndarray([nrows_in, ncols_in], data)
end

# Coerce a bare Ruby Numeric into a rank-0 (scalar) SirNDArray; an
# already-SirNDArray value passes through unchanged. Needed because
# `matlab-to-semantic-ir`'s lowerer emits a mixed operand pair for `.* ./
# .\` and for `* /` when exactly one side is scalar (e.g. `A .* 2`) — the
# BARE scalar sub-expression is passed through `ElementwiseOp` unwrapped
# (a plain Integer/Float), not wrapped in an `ArrayLit` first. Every
# function below that accepts an "array" operand normalizes through this
# first, so a raw number never reaches `.data`/`.shape` and raises
# `NoMethodError` instead of behaving correctly.
def sir_array_to_array_value(v)
  v.is_a?(SirNDArray) ? v : SirNDArray.new([], [v])
end

def sir_array_is_scalar(a) = a.data.length == 1

# Rows, treating a scalar as `1x1` and a vector `[n]` as `nx1`.
def sir_array_nrows(a) = a.shape.empty? ? 1 : a.shape[0]

# Columns, treating a scalar as `1x1` and a vector `[n]` as `nx1`.
def sir_array_ncols(a) = a.shape.length <= 1 ? 1 : a.shape[1]

# Element `(r, c)` (column-major), or `nil` if out of bounds.
#
# SECURITY: written as the AND-form `r >= 0 && c >= 0 && r < nrows && c <
# ncols`, NOT the negation `r < 0 || c < 0 || ...` — under IEEE-754 those
# are NOT equivalent for NaN: every relational comparison with NaN is
# false, so a NaN `r`/`c` would make every branch of the OR-form false too,
# silently skipping the bounds check (mirrors the JS backend's own
# `resolvePositions`/`assertValidPosition` fix for the identical hazard).
def sir_array_get(a, r, c)
  if r >= 0 && c >= 0 && r < sir_array_nrows(a) && c < sir_array_ncols(a)
    a.data[c * sir_array_nrows(a) + r]
  end
end

# Set element `(r, c)` IN PLACE (column-major) — mutates `a.data`
# directly, matching MATLAB assignment semantics (`A(i,j) = v` rebinds one
# element of the existing array, it does not produce a new one). This is
# why `Stmt::IndexSet` is a statement, not a pure expression, in the SIR22
# spec. Same NaN-safe AND-form bounds check as `sir_array_get`.
def sir_array_set(a, r, c, value)
  unless r >= 0 && c >= 0 && r < sir_array_nrows(a) && c < sir_array_ncols(a)
    raise "sir_array_set: index (#{r}, #{c}) out of bounds for shape #{a.shape}"
  end
  a.data[c * sir_array_nrows(a) + r] = value
end

# Comparisons follow the same APL-style boolean convention
# `array_runtime::BinOp` uses: `1` for true, `0` for false (never a native
# `true`/`false`), since the result must stay a plain array element like
# every other value here. `Div`/`Pow` force a Float result (Ruby's native
# Integer `/` floors and `**` can promote to Rational for a negative
# exponent — neither matches this domain's always-real-division /
# always-real-power semantics), matching JS's `Float64Array`-backed
# `Math.pow`/`/` exactly; `Add`/`Sub`/`Mul`/`Max`/`Min` preserve whatever
# Numeric type the operands already are.
def sir_array_apply_op(op, a, b)
  case op
  when "Add" then a + b
  when "Sub" then a - b
  when "Mul" then a * b
  when "Div" then a.to_f / b.to_f
  when "Pow" then a.to_f**b.to_f
  when "Max" then [a, b].max
  when "Min" then [a, b].min
  when "Eq" then a == b ? 1 : 0
  when "Ne" then a != b ? 1 : 0
  when "Lt" then a < b ? 1 : 0
  when "Le" then a <= b ? 1 : 0
  when "Ge" then a >= b ? 1 : 0
  when "Gt" then a > b ? 1 : 0
  else
    raise "sir_array_apply_op: unrecognised ElementwiseOpKind #{op.inspect}"
  end
end

def sir_array_same_shape(a, b)
  a.length == b.length && a.each_with_index.all? { |d, i| d == b[i] }
end

# Elementwise binary op with scalar broadcasting. Either operand may be a
# scalar; otherwise the shapes must match exactly (full NumPy/MATLAB
# broadcasting is out of scope, same as the Rust reference). Result takes
# the non-scalar operand's shape (or the scalar's, if both are).
def sir_array_elementwise(op, a, b)
  a = sir_array_to_array_value(a)
  b = sir_array_to_array_value(b)
  ad = a.data
  bd = b.data
  if sir_array_is_scalar(a)
    data = bd.map { |y| sir_array_apply_op(op, ad[0], y) }
  elsif sir_array_is_scalar(b)
    data = ad.map { |x| sir_array_apply_op(op, x, bd[0]) }
  else
    unless sir_array_same_shape(a.shape, b.shape)
      raise "sir_array_elementwise: non-conformable arrays: #{a.shape} vs #{b.shape}"
    end
    data = ad.each_with_index.map { |x, i| sir_array_apply_op(op, x, bd[i]) }
  end
  shape = sir_array_is_scalar(a) ? b.shape : a.shape
  sir_array_ndarray(shape, data)
end

# Matrix product `[m, k] . [k, n] -> [m, n]` (column-major throughout).
# `m`/`n` come from two INDEPENDENT operands (each individually under
# `SIR_ARRAY_MAX_ELEMENTS`, but their product isn't bounded by that alone
# — an outer-product-shaped call could still ask for a huge output), so
# `sir_array_checked_shape_size` validates `[m, n]` BEFORE allocating
# `out`, not after. Normalizes both operands through
# `sir_array_to_array_value` first, same reasoning as `sir_array_elementwise`.
def sir_array_matmul(a, b)
  a = sir_array_to_array_value(a)
  b = sir_array_to_array_value(b)
  m = sir_array_nrows(a)
  ka = sir_array_ncols(a)
  kb = sir_array_nrows(b)
  n = sir_array_ncols(b)
  if ka != kb
    raise "sir_array_matmul: inner dimensions disagree (#{m}x#{ka} . #{kb}x#{n})"
  end
  out_len = sir_array_checked_shape_size([m, n])
  ad = a.data
  bd = b.data
  out = Array.new(out_len, 0)
  (0...n).each do |j|
    (0...m).each do |i|
      acc = 0
      (0...ka).each do |p|
        acc += ad[p * m + i] * bd[j * kb + p] # column-major indexing
      end
      out[j * m + i] = acc
    end
  end
  sir_array_ndarray([m, n], out)
end

# Matrix transpose. `conjugate` distinguishes MATLAB `'` (`true`) from `.'`
# (`false`) — this runtime has no Complex value type yet (matching
# `array-runtime`'s own real-only scope today), so a conjugate transpose
# of real data is identical to a plain transpose; `conjugate` is accepted
# for call-shape parity with the SIR spec only.
def sir_array_transpose(a, _conjugate)
  m = sir_array_nrows(a)
  n = sir_array_ncols(a)
  ad = a.data
  out = Array.new(ad.length)
  (0...n).each do |j|
    (0...m).each do |i|
      out[i * n + j] = ad[j * m + i]
    end
  end
  sir_array_ndarray([n, m], out)
end

# Materialize a MATLAB-style range `start:step:stop` (default `step = 1`)
# as a `1xn` row vector — MATLAB's `:` always produces a row, never a
# column. Bounded by `SIR_ARRAY_MAX_ELEMENTS` so a compiled program's
# `1:1e18`-style range can't exhaust memory before this function ever gets
# to materialize anything. `SIR_ARRAY_RANGE_EPSILON` tolerates the
# inclusive-stop boundary (a floating step, e.g. `1:0.1:2`, can drift a
# few ULPs short of `stop` by the final iteration).
def sir_array_range(start, stop, step = 1)
  raise "sir_array_range: step cannot be zero" if step == 0
  unless sir_array_finite?(start) && sir_array_finite?(stop) && sir_array_finite?(step)
    raise "sir_array_range: start/stop/step must be finite numbers, got (#{start}, #{stop}, #{step})"
  end
  values = []
  x = start
  while (step > 0 && x <= stop + SIR_ARRAY_RANGE_EPSILON) || (step < 0 && x >= stop - SIR_ARRAY_RANGE_EPSILON)
    if values.length >= SIR_ARRAY_MAX_ELEMENTS
      raise "sir_array_range: produces more than #{SIR_ARRAY_MAX_ELEMENTS} elements"
    end
    values << x
    x += step
  end
  sir_array_ndarray(values.empty? ? [1, 0] : [1, values.length], values)
end

# One MATLAB-style index-position argument, mirroring the SIR22 spec's
# `IndexArg` exactly: `{kind: "scalar", value:}` / `{kind: "whole"}` /
# `{kind: "range", indices:}`. `end`-relative indices are never seen here
# — per SIR10 discipline, the frontend resolves `end` to a concrete
# 0-based `scalar` index before emitting `IndexGet`/`IndexSet`.

# Validate one resolved position is a real, finite integer, and return it
# as a genuine Ruby Integer (JS can index a typed array with a
# float-valued-but-integer number like `3.0` directly; Ruby's `Array#[]`
# cannot, so this coerces after validating, unlike the JS reference).
#
# SECURITY: the caller resolves EVERY position through this single choke
# point before it ever reaches `sir_array_get`/`sir_array_set`'s own
# NaN-safe bounds checks below — an unvalidated Float::NAN/Infinity index
# must fail loudly here (a clean, catchable exception), not silently
# produce a bogus position that then defeats a downstream comparison.
def sir_array_assert_valid_position(i)
  unless sir_array_integer_dim?(i)
    raise "sir_array_resolve_positions: index #{i} is not a finite integer"
  end
  i.to_i
end

def sir_array_resolve_positions(arg, dim_size)
  case arg[:kind]
  when "scalar" then [sir_array_assert_valid_position(arg[:value])]
  when "whole" then (0...dim_size).to_a
  when "range"
    arg[:indices].data.map do |x|
      # `Float#truncate` raises FloatDomainError on NaN/Infinity in Ruby
      # (unlike JS's `Math.trunc`, which returns NaN/Infinity unchanged) —
      # only truncate a value already known finite; a non-finite x is
      # passed through as-is so `sir_array_assert_valid_position` reports
      # its own clean error instead of a raw FloatDomainError escaping.
      sir_array_assert_valid_position(sir_array_finite?(x) ? x.truncate : x)
    end
  else
    raise "sir_array_resolve_positions: unrecognised IndexArg #{arg.inspect}"
  end
end

# `A(i)` / `A(i, j)` — read one element or a sub-array. Scoped to 1 or 2
# index arguments (rank <= 2): a single argument indexes `a`'s underlying
# column-major data linearly (MATLAB's own single-subscript convention,
# which is column-major too); two arguments index `(row, col)`. Returns a
# bare Numeric when every argument is `scalar` (a single element),
# otherwise a SirNDArray.
def sir_array_index_get(a, indices)
  if indices.length == 1
    arg = indices[0]
    positions = sir_array_resolve_positions(arg, a.data.length)
    read = lambda do |i|
      raise "sir_array_index_get: linear index #{i} out of bounds" if i < 0 || i >= a.data.length
      a.data[i]
    end
    return read.call(positions[0]) if arg[:kind] == "scalar"
    return sir_array_ndarray([1, positions.length], positions.map { |i| read.call(i) })
  end
  if indices.length == 2
    row_arg, col_arg = indices
    rows = sir_array_resolve_positions(row_arg, sir_array_nrows(a))
    cols = sir_array_resolve_positions(col_arg, sir_array_ncols(a))
    read = lambda do |r, c|
      v = sir_array_get(a, r, c)
      raise "sir_array_index_get: (#{r}, #{c}) out of bounds for shape #{a.shape}" if v.nil?
      v
    end
    return read.call(rows[0], cols[0]) if row_arg[:kind] == "scalar" && col_arg[:kind] == "scalar"
    # `rows.length`/`cols.length` are each individually bounded by `a`'s
    # own dimensions (`whole`) or by a `range` NDArray's own
    # SIR_ARRAY_MAX_ELEMENTS cap — but nothing bounds their PRODUCT on its
    # own, so this is the exact outer-product-shaped allocation
    # `sir_array_matmul` guards against, one level up. Validate before
    # allocating, not after.
    out_len = sir_array_checked_shape_size([rows.length, cols.length])
    data = Array.new(out_len)
    cols.each_with_index do |c, ci|
      rows.each_with_index do |r, ri|
        data[ci * rows.length + ri] = read.call(r, c)
      end
    end
    return sir_array_ndarray([rows.length, cols.length], data)
  end
  raise "sir_array_index_get: only 1 or 2 index arguments are supported (rank <= 2 scope), got #{indices.length}"
end

# Broadcast a scalar-or-SirNDArray right-hand side to exactly `count`
# values (mirrors `sir_array_elementwise`'s scalar-broadcast rule).
def sir_array_broadcast_values(value, count)
  return Array.new(count, value) if value.is_a?(Numeric)
  return Array.new(count, value.data[0]) if value.data.length == 1
  if value.data.length != count
    raise "sir_array_index_set: value has #{value.data.length} elements, expected #{count}"
  end
  value.data
end

# `A(i) = v` / `A(i, j) = v` — write one element or a sub-array, IN PLACE
# (see `sir_array_set`'s doc comment above for why this mutates rather
# than returns a new array). `value` may be a scalar (broadcast to every
# selected position) or a SirNDArray with exactly as many elements as
# positions are selected.
def sir_array_index_set(a, indices, value)
  if indices.length == 1
    arg = indices[0]
    positions = sir_array_resolve_positions(arg, a.data.length)
    values = sir_array_broadcast_values(value, positions.length)
    positions.each_with_index do |i, k|
      raise "sir_array_index_set: linear index #{i} out of bounds" if i < 0 || i >= a.data.length
      a.data[i] = values[k]
    end
    return
  end
  if indices.length == 2
    row_arg, col_arg = indices
    rows = sir_array_resolve_positions(row_arg, sir_array_nrows(a))
    cols = sir_array_resolve_positions(col_arg, sir_array_ncols(a))
    # Same product-of-two-independent-selections gap `sir_array_index_get`
    # closes above — validate before `sir_array_broadcast_values` allocates.
    count = sir_array_checked_shape_size([rows.length, cols.length])
    values = sir_array_broadcast_values(value, count)
    k = 0
    cols.each do |c|
      rows.each do |r|
        sir_array_set(a, r, c, values[k])
        k += 1
      end
    end
    return
  end
  raise "sir_array_index_set: only 1 or 2 index arguments are supported (rank <= 2 scope), got #{indices.length}"
end

# ── SIR22 addendum: APL primitive operators (Phase A Slice 3) ──
#
# `Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/`IndexGenerator`/
# `IndexOf`/`Ravel`/`Catenate` lower to the `sir_array_*` functions below —
# an inlined port of `semantic-ir-to-javascript`'s own already-proven
# addendum functions (`runtime.rs`'s own "SIR22 addendum: APL primitive
# operators" section, itself ported 1:1 from `apl-runtime::builtins` /
# `array_runtime::ops`), following the exact same naming/style convention
# as the base cut above. `SIR_ARRAY_MAX_ELEMENTS` (defined above) is reused
# as-is for every new bounded-allocation check here — this file has exactly
# one array-size cap, not one per domain.

# `+/A` (APL reduce, dyadic-op monadic-adverb) — fold `target` with `op`
# along its one axis. Ported 1:1 from `array_runtime::ops::reduce`:
# - rank 0 (scalar): nothing to fold, returns `target` itself.
# - rank 1 (vector `[n]`): left-fold across all `n` elements
#   (`op(op(op(v0, v1), v2), ...)`); an EMPTY vector is a clean error --
#   unlike `sum`/`mean` (which have a built-in identity, 0), `reduce` is
#   generic over any `op`, and guessing an identity (is it `0` for `Add`,
#   `1` for `Mul`, `-Infinity` for `Max`?) for an arbitrary, possibly-future
#   op would be silently wrong for most of them.
# - rank 2 (matrix `[r, c]`): folds EACH ROW independently across its `c`
#   columns, producing a `[r]` vector (one folded value per row).
#   Column-major storage means element `(row, col)` lives at
#   `col * r + row` -- the row loop reads `d[row]` as the seed (column 0)
#   then walks `d[col * r + row]` for `col = 1..c`; getting `row` and `col`
#   swapped here silently transposes the result instead of raising, so this
#   indexing is the single easiest place to introduce a wrong-answer bug
#   when reading this function.
def sir_array_reduce(op, a)
  a = sir_array_to_array_value(a)
  shape = a.shape
  return a if shape.empty?
  if shape.length == 1
    n = shape[0]
    raise "sir_array_reduce: cannot fold an empty vector (no identity element for an arbitrary op)" if n == 0
    d = a.data
    acc = d[0]
    (1...n).each { |i| acc = sir_array_apply_op(op, acc, d[i]) }
    return sir_array_ndarray([], [acc])
  end
  if shape.length == 2
    r, c = shape
    raise "sir_array_reduce: cannot fold an empty row (no identity element for an arbitrary op)" if c == 0
    d = a.data
    out = Array.new(r)
    (0...r).each do |row|
      acc = d[row] # column-major: (row, 0) lives at plain `row`
      (1...c).each { |col| acc = sir_array_apply_op(op, acc, d[col * r + row]) }
      out[row] = acc
    end
    return sir_array_ndarray([r], out)
  end
  raise "sir_array_reduce: rank > 2 not yet supported (shape #{shape})"
end

# `+\A` (APL scan) — the same fold as `sir_array_reduce`, but keeping EVERY
# intermediate result instead of only the last; output has the same shape
# as `target`. Ported 1:1 from `array_runtime::ops::scan`. An empty axis is
# NOT an error here (unlike `reduce`): there is simply nothing to scan, and
# the (empty) output shape already says so.
def sir_array_scan(op, a)
  a = sir_array_to_array_value(a)
  shape = a.shape
  return a if shape.empty?
  if shape.length == 1
    n = shape[0]
    d = a.data
    out = Array.new(n)
    acc = nil
    started = false
    (0...n).each do |i|
      acc = started ? sir_array_apply_op(op, acc, d[i]) : d[i]
      started = true
      out[i] = acc
    end
    return sir_array_ndarray([n], out)
  end
  if shape.length == 2
    r, c = shape
    d = a.data
    out = Array.new(d.length)
    (0...r).each do |row|
      acc = nil
      started = false
      (0...c).each do |col|
        x = d[col * r + row] # column-major
        acc = started ? sir_array_apply_op(op, acc, x) : x
        started = true
        out[col * r + row] = acc
      end
    end
    return sir_array_ndarray([r, c], out)
  end
  raise "sir_array_scan: rank > 2 not yet supported (shape #{shape})"
end

# `A∘.×B` (APL outer product) — apply `op` to every pair `(aᵢ, bⱼ)`,
# producing a result of rank `rank(a) + rank(b)`. Ported 1:1 from
# `array_runtime::ops::outer`, scoped identically to `rank(a) <= 1` and
# `rank(b) <= 1` (the vector⊗vector case below already reaches this
# domain's rank-2 ceiling). `sir_array_checked_shape_size` validates the
# `[m, n]` output shape *before* allocating — `m`/`n` are two INDEPENDENT
# operand lengths, each individually under `SIR_ARRAY_MAX_ELEMENTS`, but
# nothing bounds their product alone (the same outer-product-shaped
# allocation `sir_array_matmul`/`sir_array_index_get` above guard).
def sir_array_outer(op, a, b)
  a = sir_array_to_array_value(a)
  b = sir_array_to_array_value(b)
  as = a.shape
  bs = b.shape
  if as.empty? && bs.empty?
    return sir_array_ndarray([], [sir_array_apply_op(op, a.data[0], b.data[0])])
  end
  if as.empty? && bs.length == 1
    x = a.data[0]
    return sir_array_ndarray([bs[0]], b.data.map { |y| sir_array_apply_op(op, x, y) })
  end
  if as.length == 1 && bs.empty?
    y = b.data[0]
    return sir_array_ndarray([as[0]], a.data.map { |x| sir_array_apply_op(op, x, y) })
  end
  if as.length == 1 && bs.length == 1
    m = as[0]
    n = bs[0]
    out_len = sir_array_checked_shape_size([m, n])
    ad = a.data
    bd = b.data
    out = Array.new(out_len)
    (0...n).each do |j|
      (0...m).each { |i| out[j * m + i] = sir_array_apply_op(op, ad[i], bd[j]) } # column-major
    end
    return sir_array_ndarray([m, n], out)
  end
  raise "sir_array_outer: operands of rank > 1 not yet supported (shapes #{as}, #{bs})"
end

# Flatten (rank <= 2, this domain's ceiling) `a` to ROW-major order — last
# axis varies fastest. `a` itself stores COLUMN-major
# (`sir_array_get`'s own doc comment), so a matrix must be walked "row,
# then column" via `sir_array_get` to produce true row-major order;
# returning the raw column-major buffer would silently ravel in the WRONG
# order. Always returns a FRESH Array (`.dup`, never `a.data` itself, even
# in the rank <= 1 no-op case) — mirrors `apl_runtime::builtins::flatten`
# returning an owned `Vec`, not a borrow, so the result never accidentally
# aliases `a`'s own buffer (a caller mutating the returned Array must not
# also mutate `a`).
def sir_array_flatten_row_major(a)
  shape = a.shape
  return a.data.dup if shape.length <= 1
  if shape.length == 2
    r, c = shape
    out = Array.new(r * c)
    k = 0
    (0...r).each do |row|
      (0...c).each do |col|
        out[k] = sir_array_get(a, row, col)
        k += 1
      end
    end
    return out
  end
  # Unreachable in practice (this domain's rank <= 2 ceiling) -- total
  # rather than raising, mirroring the JS reference's own fallback.
  a.data.dup
end

# Monadic `⍴` (shape-of) — `target`'s dimensions as a vector. Ported 1:1
# from `apl_runtime::builtins::shape`: a SCALAR has zero dimensions, so its
# shape is the EMPTY vector (not a scalar!) — `⍴5` is a length-0 vector,
# mirroring `shape.length == 0` exactly. A vector `[n]` has shape `[n]`
# (one element); a matrix `[r, c]` has shape `[r, c]` (two elements).
def sir_array_shape(a)
  a = sir_array_to_array_value(a)
  sir_array_ndarray([a.shape.length], a.shape.dup)
end

# Dyadic `⍴` (reshape) — reinterpret `target`'s data under the new
# dimensions `shape_arg`. Ported 1:1 from `apl_runtime::builtins::reshape`.
# `shape_arg` must itself be a scalar or vector (rank <= 1) of
# non-negative integers, and is itself capped at rank <= 2 (this domain's
# ceiling — a longer target shape is a clean error, not a silent
# truncation). `target`'s elements are ravelled (`sir_array_flatten_row_major`)
# then cyclically repeated or truncated to fill the target shape's element
# count.
#
# CRITICAL: the cyclic fill happens in ROW-major order (APL's reshape
# fills the LAST axis fastest, same convention as ravel), but this
# domain's storage is COLUMN-major — so for a rank-2 target the row-major
# `filled` sequence must be TRANSPOSED into column-major storage
# (`data[col * r + row] = filled[row * c + col]`) before calling
# `sir_array_ndarray`. Handing `filled` straight to `sir_array_ndarray`
# would silently reshape column-major instead of APL's row-major
# convention — a wrong answer that still LOOKS plausible (right multiset
# of values, wrong positions).
def sir_array_reshape(shape_arg, target)
  shape_arg = sir_array_to_array_value(shape_arg)
  target = sir_array_to_array_value(target)
  if shape_arg.shape.length > 1
    raise "sir_array_reshape: shape argument must be a scalar or vector (got rank #{shape_arg.shape.length})"
  end
  dims = shape_arg.data.map do |x|
    unless sir_array_integer_dim?(x) && x >= 0
      raise "sir_array_reshape: shape elements must be non-negative integers, got #{x}"
    end
    x.to_i
  end
  if dims.length > 2
    raise "sir_array_reshape: reshape to rank > 2 is not yet supported (target shape #{dims})"
  end
  total = sir_array_checked_shape_size(dims)
  source = sir_array_flatten_row_major(target)
  if total > 0 && source.empty?
    raise "sir_array_reshape: cannot reshape an empty source into a non-empty shape"
  end
  filled = Array.new(total)
  (0...total).each { |k| filled[k] = source[k % source.length] }
  return sir_array_ndarray(dims, filled) if dims.length <= 1
  r, c = dims
  data = Array.new(total)
  (0...r).each do |row|
    (0...c).each do |col|
      data[col * r + row] = filled[row * c + col]
    end
  end
  sir_array_ndarray(dims, data)
end

# Monadic `⍳` (index generator / iota) — `⍳n` is the 1-BASED vector
# `[1, 2, ..., n]`. Ported 1:1 from `apl_runtime::builtins::
# index_generator` — note this is 1-based, unlike every 0-based index
# elsewhere in this domain (`sir_array_index_get`/`sir_array_index_set`),
# because that is genuinely what APL's `⍳` means at the SURFACE-SYNTAX
# level (see `apl-runtime`'s own tests, e.g.
# `index_generator_produces_one_based_run`). `sir_array_checked_shape_size`
# both validates `n` is a non-negative integer AND caps it at
# `SIR_ARRAY_MAX_ELEMENTS` before allocating — `n` is a runtime value a
# compiled program computes, not a fixed constant, so `⍳` of an absurd size
# must fail cleanly.
def sir_array_index_generator(a)
  a = sir_array_to_array_value(a)
  raise "sir_array_index_generator: monadic argument must be a scalar" unless sir_array_is_scalar(a)
  x = a.data[0]
  unless sir_array_integer_dim?(x) && x >= 0
    raise "sir_array_index_generator: monadic argument must be a non-negative integer, got #{x}"
  end
  n = sir_array_checked_shape_size([x.to_i])
  sir_array_ndarray([n], Array.new(n) { |i| i + 1 })
end

# Dyadic `⍳` (index-of / search) — for every element of `needle`, the
# 1-based index of its first occurrence in the vector `haystack` (or
# `haystack.length + 1` if not found — "not found" is a valid,
# always-in-range position, not `-1`/`nil`). Ported 1:1 from
# `apl_runtime::builtins::index_of`: plain EXACT equality (`Array#index`
# uses `==`, so `Float::NAN` correctly never matches itself, same as
# Rust's `==`). The work done is O(len(haystack) * len(needle)) (a full
# linear scan per needle element) — `sir_array_checked_shape_size` is
# reused here purely for its "product <= SIR_ARRAY_MAX_ELEMENTS" check
# (both lengths are already valid non-negative integers, so its
# dimension-validity half is a no-op) to cap the PRODUCT before scanning,
# since each operand individually staying under `SIR_ARRAY_MAX_ELEMENTS`
# does not bound their product (up to ~4.5 * 10^15 comparisons otherwise).
def sir_array_index_of(a, b)
  a = sir_array_to_array_value(a)
  b = sir_array_to_array_value(b)
  if a.shape.length > 1
    raise "sir_array_index_of: left argument must be a scalar or vector (got rank #{a.shape.length})"
  end
  sir_array_checked_shape_size([a.data.length, b.data.length])
  haystack = a.data
  out = b.data.map do |needle|
    idx = haystack.index(needle)
    idx.nil? ? haystack.length + 1 : idx + 1
  end
  sir_array_ndarray(b.shape, out)
end

# Monadic `,` (ravel) — flatten `target` to a rank-1 vector, in row-major
# order (see `sir_array_flatten_row_major`'s own doc comment for the
# column-major-storage-vs-row-major-order subtlety). Ported 1:1 from
# `apl_runtime::builtins::ravel`.
def sir_array_ravel(a)
  a = sir_array_to_array_value(a)
  flat = sir_array_flatten_row_major(a)
  sir_array_ndarray([flat.length], flat)
end

# Dyadic `,` (catenate) — supports scalar-scalar, scalar-vector,
# vector-scalar, vector-vector (all producing a vector), and
# matrix-matrix-with-equal-row-counts (column/last-axis catenate,
# producing `[r, ca + cb]`). Any other rank combination is a clean "not
# yet supported" error. Ported 1:1 from `apl_runtime::builtins::catenate`.
# The combined-length cap check happens ONCE, up front, regardless of
# which rank combination follows (mirroring the Rust reference's own
# structure) — neither operand alone need be oversized for the RESULT to
# be, since a script that repeatedly catenates a value with itself
# (`A <- A,A`) doubles the size every line with no other ceiling.
def sir_array_catenate(a, b)
  a = sir_array_to_array_value(a)
  b = sir_array_to_array_value(b)
  sir_array_checked_shape_size([a.data.length + b.data.length])
  ra = a.shape.length
  rb = b.shape.length
  if ra == 0 && rb == 0
    return sir_array_ndarray([2], [a.data[0], b.data[0]])
  end
  if ra == 0 && rb == 1
    out = [a.data[0]] + b.data
    return sir_array_ndarray([out.length], out)
  end
  if ra == 1 && rb == 0
    out = a.data + [b.data[0]]
    return sir_array_ndarray([out.length], out)
  end
  if ra == 1 && rb == 1
    out = a.data + b.data
    return sir_array_ndarray([out.length], out)
  end
  if ra == 2 && rb == 2
    r = sir_array_nrows(a)
    unless r == sir_array_nrows(b)
      raise "sir_array_catenate: matrix catenate needs equal row counts (#{r} vs #{sir_array_nrows(b)})"
    end
    ca = sir_array_ncols(a)
    cb = sir_array_ncols(b)
    out_len = sir_array_checked_shape_size([r, ca + cb])
    data = Array.new(out_len)
    (0...r).each do |row|
      (0...ca).each { |col| data[col * r + row] = sir_array_get(a, row, col) }
      (0...cb).each { |col| data[(ca + col) * r + row] = sir_array_get(b, row, col) }
    end
    return sir_array_ndarray([r, ca + cb], data)
  end
  raise "sir_array_catenate: catenate of rank #{ra} and rank #{rb} is not yet supported"
end

# ── SIR23 symbolic expressions + pattern/rewrite (Tier A, Phase A Slice 4) ──
#
# `SymSymbol`/`SymRational`/`SymApply`/`SymPatternBlank`/`SymPatternNamed`/
# `SymRule`/`SymReplaceAll` lower to calls into `sir_sym_*` below — an
# inlined port of `semantic-ir-to-javascript`'s own already-proven
# `Symbolic` sub-runtime's Tier A (matcher) slice: term construction,
# `matchPattern`/`substituteTerm`/`applyRuleTerm`, and `replaceAll`/
# `replaceRepeated` with their `MAX_TERM_DEPTH` guard. Tier B (`evalTerm`,
# the arithmetic/calculus/user-function evaluator) is explicitly OUT OF
# SCOPE for this slice — matching the SIR23 spec's own Tier A/Tier B split
# — so no `Add`/`Sin`/`D`/... folding exists here; a `SymApply` builds an
# inert term tree, nothing more.
#
# ## Value model
#
# `SirSymTerm(kind, name, value, numer, denom, head, args)` — one Struct
# type discriminated by `.kind` (a Ruby Symbol: `:symbol`/`:integer`/
# `:rational`/`:float`/`:string`/`:apply`), mirroring the JS reference's
# frozen `{kind, ...}` object shape field-for-field. Only the fields a
# given `kind` uses are populated; the rest stay `nil`. Every constructor
# below returns a `.freeze`d instance, so a term is immutable once built —
# matching the JS reference's `Object.freeze` and this domain's own
# persistent/copy-on-write bindings model (a failed match attempt never
# mutates a binding set an earlier attempt still holds).
#
# Unlike the JS reference (which restricts `Symbolic.int`/`.rational` to
# `Number.isSafeInteger` because JS's own `Expr::IntLit` codegen already
# has that ceiling), Ruby Integers are arbitrary-precision, and this
# backend's own `Expr::IntLit` arm already emits a bare unbounded Ruby
# integer literal — so `sir_sym_int` accepts any Ruby Integer with no
# extra range check.
SirSymTerm = Struct.new(:kind, :name, :value, :numer, :denom, :head, :args)

# Generic term -> String rendering (`head(args, ...)`), used by `sir_fmt`
# (below, via a `when SirSymTerm` case arm) so `print`/`puts` on a term
# behaves like every other displayable SIR value. A direct, minimal port
# of the JS reference's `toDisplayString`'s generic (non-`SIR_DISPLAY_
# DERIVE`) branch ONLY — the Derive-specific infix/precedence pretty-
# printer (SIR23 addendum item 4) is separate follow-up work, not part of
# this Tier A matcher port. Depth-capped with the same
# `SIR_SYM_MAX_TERM_DEPTH` guard the matcher's own tree walks use, for the
# identical reason: a term built via `sir_sym_apply` is not depth-capped
# at construction time, so a runtime-built term can be arbitrarily deep.
def sir_sym_to_s(node, depth = 0)
  return "..." if depth > SIR_SYM_MAX_TERM_DEPTH
  case node.kind
  when :symbol then node.name
  when :integer then node.value.to_s
  when :rational then "#{node.numer}/#{node.denom}"
  when :float then sir_fmt_float(node.value)
  when :string then node.value.inspect
  when :apply
    "#{sir_sym_to_s(node.head, depth + 1)}(#{node.args.map { |a| sir_sym_to_s(a, depth + 1) }.join(', ')})"
  else
    node.to_s
  end
end

def sir_sym_symbol(name) = SirSymTerm.new(:symbol, name).freeze

def sir_sym_int(value)
  raise "sir_sym_int: value must be an Integer" unless value.is_a?(Integer)
  SirSymTerm.new(:integer, nil, value).freeze
end

def sir_sym_gcd_abs(a, b)
  a = a.abs
  b = b.abs
  a, b = b, a % b while b != 0
  a.zero? ? 1 : a
end

def sir_sym_rational(numer, denom)
  raise "sir_sym_rational: denominator cannot be zero" if denom == 0
  if denom < 0
    numer = -numer
    denom = -denom
  end
  g = sir_sym_gcd_abs(numer, denom)
  SirSymTerm.new(:rational, nil, nil, numer / g, denom / g).freeze
end

def sir_sym_float(value)
  raise "sir_sym_float: value must be finite" unless value.is_a?(Float) && value.finite?
  SirSymTerm.new(:float, nil, value).freeze
end

def sir_sym_string(value) = SirSymTerm.new(:string, nil, value).freeze

def sir_sym_apply(head, args) = SirSymTerm.new(:apply, nil, nil, nil, nil, head, args.dup.freeze).freeze

# Structural equality — used by the matcher (a repeated pattern variable
# must bind to the SAME term every occurrence), by `sir_sym_bindings_bind`'s
# "already bound to this exact term" fast path, and by
# `sir_sym_replace_repeated`'s "did this firing actually change anything"
# fixed-point check.
#
# SECURITY (CWE-674): depth-capped with the same `SIR_SYM_MAX_TERM_DEPTH`
# guard `sir_sym_walk_once`/`sir_sym_replace_repeated` use below — a term
# built via `sir_sym_apply` is not depth-capped at CONSTRUCTION time (only
# the tree WALK below enforces the cap), so an attacker-influenced runtime
# value (e.g. a loop lowered to repeated `SymApply` nesting) could build
# one arbitrarily deep directly. Past the cap, `false` ("not structurally
# equal") is the safe, contained answer, mirroring `sir_sym_match_pattern`'s
# own "give up cleanly" contract for a failed match.
def sir_sym_term_equals(a, b, depth = 0)
  return false if depth > SIR_SYM_MAX_TERM_DEPTH
  return false if a.kind != b.kind
  case a.kind
  when :symbol then a.name == b.name
  when :integer then a.value == b.value
  when :rational then a.numer == b.numer && a.denom == b.denom
  when :float then a.value == b.value
  when :string then a.value == b.value
  when :apply
    sir_sym_term_equals(a.head, b.head, depth + 1) &&
      a.args.length == b.args.length &&
      a.args.each_with_index.all? { |arg, i| sir_sym_term_equals(arg, b.args[i], depth + 1) }
  else
    false
  end
end

def sir_sym_head_name(node) = node.kind == :symbol ? node.name : ""

# ── pattern/rule vocabulary (cas-pattern-matching) ──────────────────────
SIR_SYM_BLANK = "Blank"
SIR_SYM_PATTERN = "Pattern"
SIR_SYM_RULE = "Rule"
SIR_SYM_RULE_DELAYED = "RuleDelayed"

def sir_sym_is_head?(node, name)
  node.kind == :apply && node.head.kind == :symbol && node.head.name == name
end
def sir_sym_is_blank?(node) = sir_sym_is_head?(node, SIR_SYM_BLANK)
def sir_sym_is_pattern?(node) = sir_sym_is_head?(node, SIR_SYM_PATTERN)
def sir_sym_is_rule?(node)
  node.kind == :apply && node.head.kind == :symbol &&
    (node.head.name == SIR_SYM_RULE || node.head.name == SIR_SYM_RULE_DELAYED) &&
    node.args.length == 2
end

def sir_sym_blank() = sir_sym_apply(sir_sym_symbol(SIR_SYM_BLANK), [])
def sir_sym_blank_typed(head) = sir_sym_apply(sir_sym_symbol(SIR_SYM_BLANK), [sir_sym_symbol(head)])
def sir_sym_named(name, inner) = sir_sym_apply(sir_sym_symbol(SIR_SYM_PATTERN), [sir_sym_symbol(name), inner])
def sir_sym_rule(lhs, rhs) = sir_sym_apply(sir_sym_symbol(SIR_SYM_RULE), [lhs, rhs])
def sir_sym_rule_delayed(lhs, rhs) = sir_sym_apply(sir_sym_symbol(SIR_SYM_RULE_DELAYED), [lhs, rhs])

# Bindings: a name -> term Hash. Persistent / copy-on-write (mirrors
# `cas-pattern-matching`'s `Bindings` class) so a failed match attempt
# never mutates a binding set an earlier attempt still holds a reference
# to — `sir_sym_bindings_bind` always returns a NEW Hash, never mutates
# `bindings` in place.
def sir_sym_bindings_empty() = {}
def sir_sym_bindings_bind(bindings, name, value)
  existing = bindings[name]
  return bindings if !existing.nil? && sir_sym_term_equals(existing, value)
  bindings.merge(name => value)
end

def sir_sym_blank_head_constraint(node)
  return nil if node.args.empty?
  first = node.args[0]
  first.kind == :symbol ? first.name : nil
end
def sir_sym_pattern_name(node)
  first = node.args[0]
  raise "Symbolic: Pattern name must be a Symbol" if first.nil? || first.kind != :symbol
  first.name
end
def sir_sym_pattern_inner(node)
  raise "Symbolic: Pattern requires an inner expression" if node.args.length < 2
  node.args[1]
end
def sir_sym_effective_head_name(node)
  if node.kind == :apply
    hn = sir_sym_head_name(node.head)
    return hn.empty? ? "Apply" : hn
  end
  case node.kind
  when :integer then "Integer"
  when :rational then "Rational"
  when :float then "Float"
  when :string then "String"
  else "Symbol"
  end
end

# Five-case structural matcher: `Blank()`, `Blank(T)`, `Pattern(name,
# inner)`, compound-vs-compound (recurse head + every arg, same arity
# required), and plain structural equality — a direct port of
# `cas-pattern-matching::matchPattern`.
#
# SECURITY (CWE-674, /security-review finding, applied as a follow-up fix
# after this function shipped depth-uncapped): this function's own
# original doc comment (and the JS reference it was ported from) argued no
# cap was needed because a rule's `lhs`/`rhs` is always "author-written,
# not runtime-controlled" and therefore shallow. That premise does not
# actually hold in ANY of this arc's backends: `Expr::SymRule`'s `lhs`/
# `rhs` are ordinary `Expr`s — `emit_sym_operand`'s catch-all passes a
# `VarRef` through unchanged, so a rule's pattern/template can be a local
# variable holding a term a compiled `for`-loop built to unbounded depth
# at RUNTIME, identical in kind to the target-tree hazard `sir_sym_
# replace_all`/`replace_repeated` already guard against. `depth` mirrors
# the SAME `SIR_SYM_MAX_TERM_DEPTH` cap those two use, reset to 0 at each
# fresh `sir_sym_apply_rule` call so one rule's match/substitute gets its
# own independent depth budget. Past the cap, raises the SAME "sir-
# runtime-symbolic: depth-limit" error `sir_sym_unwrap` already raises for
# the target-tree walk guards — NOT a silent `nil`/truncated fallback: a
# silently wrong-but-plausible match/substitution result would be worse
# than a loud, catchable failure (this repo's own standing "never trade
# loud for silent" discipline for a safety-relevant path).
def sir_sym_match_pattern(pattern, target, bindings = sir_sym_bindings_empty, depth = 0)
  raise "sir-runtime-symbolic: depth-limit" if depth > SIR_SYM_MAX_TERM_DEPTH
  if sir_sym_is_blank?(pattern)
    constraint = sir_sym_blank_head_constraint(pattern)
    return bindings if constraint.nil?
    return sir_sym_effective_head_name(target) == constraint ? bindings : nil
  end
  if sir_sym_is_pattern?(pattern)
    name = sir_sym_pattern_name(pattern)
    inner = sir_sym_pattern_inner(pattern)
    matched = sir_sym_match_pattern(inner, target, bindings, depth + 1)
    return nil if matched.nil?
    existing = matched[name]
    return sir_sym_term_equals(existing, target) ? matched : nil unless existing.nil?
    return sir_sym_bindings_bind(matched, name, target)
  end
  if pattern.kind == :apply
    return nil unless target.kind == :apply
    current = sir_sym_match_pattern(pattern.head, target.head, bindings, depth + 1)
    return nil if current.nil?
    return nil if pattern.args.length != target.args.length
    pattern.args.each_with_index do |p, i|
      current = sir_sym_match_pattern(p, target.args[i], current, depth + 1)
      return nil if current.nil?
    end
    return current
  end
  sir_sym_term_equals(pattern, target) ? bindings : nil
end

# SECURITY (CWE-674): depth-capped for the identical reason `sir_sym_
# match_pattern` above now is — a rule's RHS (`template` here) is subject
# to the exact same "runtime-built, not necessarily shallow" hazard, and
# is reached EVEN WHEN the target being rewritten is itself shallow (e.g.
# `Blank() -> <a 600-deep term>` matches a bare one-node target instantly,
# then `substitute` still has to rebuild the WHOLE deep RHS) — so this
# cannot be caught by `sir_sym_walk_once`/`replace_repeated`'s own
# target-tree depth tracking alone. Raises loudly past the cap, same as
# `sir_sym_match_pattern` above, not a silent truncated fallback.
def sir_sym_substitute(template, bindings, depth = 0)
  raise "sir-runtime-symbolic: depth-limit" if depth > SIR_SYM_MAX_TERM_DEPTH
  if sir_sym_is_pattern?(template)
    captured = bindings[sir_sym_pattern_name(template)]
    return captured.nil? ? template : captured
  end
  if template.kind == :apply
    return sir_sym_apply(
      sir_sym_substitute(template.head, bindings, depth + 1),
      template.args.map { |a| sir_sym_substitute(a, bindings, depth + 1) }
    )
  end
  template
end

def sir_sym_apply_rule(rewrite_rule, expr)
  raise "Symbolic.applyRule: expected Rule/RuleDelayed" unless sir_sym_is_rule?(rewrite_rule)
  lhs = rewrite_rule.args[0]
  rhs = rewrite_rule.args[1]
  bindings = sir_sym_match_pattern(lhs, expr, sir_sym_bindings_empty, 0)
  bindings.nil? ? nil : sir_sym_substitute(rhs, bindings, 0)
end

# ── replaceAll / replaceRepeated (`/.` / `//.`) + depth guard ────────────
#
# `sir_sym_match_pattern`/`sir_sym_substitute`/`sir_sym_apply_rule` recurse,
# but only as deep as a single RULE's own (author-written, not
# runtime-controlled) pattern/RHS shape — always shallow regardless of how
# deep the TARGET expression is. `sir_sym_walk_once`/`sir_sym_replace_repeated`,
# by contrast, walk the ENTIRE target expression tree, which ordinary
# compiled-program data can build up to unbounded depth — so these two need
# an explicit cap (CWE-674 stack-overflow DoS guard), mirroring the JS
# reference's `MAX_TERM_DEPTH = 512` exactly (already cross-validated
# against the published TypeScript `sir-runtime-symbolic` package, which
# uses the identical constant).
SIR_SYM_MAX_TERM_DEPTH = 512

def sir_sym_depth_limit_error() = { kind: :depth_limit, max_depth: SIR_SYM_MAX_TERM_DEPTH }
def sir_sym_is_depth_limit_error?(v) = v.is_a?(Hash) && v[:kind] == :depth_limit
def sir_sym_rewrite_cycle_error(max_iterations) = { kind: :rewrite_cycle, max_iterations: max_iterations }
def sir_sym_is_rewrite_cycle_error?(v) = v.is_a?(Hash) && v[:kind] == :rewrite_cycle

# `expr /. rules` — one pass, bottom-up: a node's head/args are walked
# (and possibly replaced) before the node itself is tried against `rules`;
# the first matching rule wins and the freshly substituted replacement is
# NOT re-walked or retried at that same position (Wolfram's single-pass
# `/.` contract, distinct from `sir_sym_replace_repeated`'s fixed point
# below).
def sir_sym_walk_once(node, rules, depth)
  return sir_sym_depth_limit_error if depth > SIR_SYM_MAX_TERM_DEPTH
  current = node
  if node.kind == :apply
    new_head = sir_sym_walk_once(node.head, rules, depth + 1)
    return new_head if sir_sym_is_depth_limit_error?(new_head)
    new_args = node.args.map do |arg|
      next_arg = sir_sym_walk_once(arg, rules, depth + 1)
      return next_arg if sir_sym_is_depth_limit_error?(next_arg)
      next_arg
    end
    current = sir_sym_apply(new_head, new_args)
  end
  rules.each do |rule|
    replacement = sir_sym_apply_rule(rule, current)
    return replacement unless replacement.nil?
  end
  current
end

def sir_sym_replace_all(expr, rules) = sir_sym_walk_once(expr, rules, 0)

# `expr //. rules` — a fixed point: at each subtree, keep retrying `rules`
# until none fire (re-walking any fresh replacement so its own sub-parts
# also converge) before moving up to the parent. `max_iterations` (default
# 100) is a GLOBAL cap shared across the whole walk, guarding against a
# non-terminating rule set (SIR23 spec "Matcher semantics" point 6). A
# firing loops LOCALLY at the current call frame (never a recursive call
# on the replacement), so however many times a rule fires at one tree
# position costs O(1) native stack frames, not O(firings) — `depth` only
# increases on a genuine descent into `head`/`args`, so `max_iterations`
# bounds iteration COUNT (CPU time) only, never native recursion depth.
#
# `walk` is a `lambda` (not a plain block) specifically so `return` inside
# it exits only that recursive call, not the enclosing method — matching
# the JS reference's nested `function walk` closure exactly.
def sir_sym_replace_repeated(expr, rules, max_iterations = 100)
  counter = 0
  walk = lambda do |node, depth|
    return sir_sym_depth_limit_error if depth > SIR_SYM_MAX_TERM_DEPTH
    current = node
    loop do
      if current.kind == :apply
        new_head = walk.call(current.head, depth + 1)
        return new_head if sir_sym_is_depth_limit_error?(new_head) || sir_sym_is_rewrite_cycle_error?(new_head)
        new_args = current.args.map do |arg|
          next_arg = walk.call(arg, depth + 1)
          return next_arg if sir_sym_is_depth_limit_error?(next_arg) || sir_sym_is_rewrite_cycle_error?(next_arg)
          next_arg
        end
        current = sir_sym_apply(new_head, new_args)
      end
      fired = false
      rules.each do |rule|
        replacement = sir_sym_apply_rule(rule, current)
        next if replacement.nil? || sir_sym_term_equals(replacement, current)
        counter += 1
        return sir_sym_rewrite_cycle_error(max_iterations) if counter > max_iterations
        current = replacement
        fired = true
        break
      end
      return current unless fired
    end
  end
  walk.call(expr, 0)
end

# Unwrap a `sir_sym_replace_all`/`sir_sym_replace_repeated` result, raising
# if the walk hit its depth cap or (for `replace_repeated`) its iteration
# cap instead of returning a real term. Every compiled `SymReplaceAll`
# routes through this — it is an ordinary expression that must evaluate to
# a term or fail loudly, never silently hand a sentinel Hash to code
# expecting a `SirSymTerm`.
def sir_sym_unwrap(result)
  raise "sir-runtime-symbolic: depth-limit" if sir_sym_is_depth_limit_error?(result)
  raise "sir-runtime-symbolic: rewrite-cycle" if sir_sym_is_rewrite_cycle_error?(result)
  result
end
"####;
