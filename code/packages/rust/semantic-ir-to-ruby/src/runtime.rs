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
# `Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`) is deferred —
# these nine share `NDArrays`/`MatrixOps`/`ArrayColumnMajor` with the base
# cut above, so `lib.rs`'s `compile` adds a dedicated pre-emit scan
# (`ScanHit::Sir22AddendumNode`) rejecting them cleanly, beyond the
# ordinary feature-flag capability check.
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
"####;
