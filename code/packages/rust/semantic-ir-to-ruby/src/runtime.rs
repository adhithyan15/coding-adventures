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

# `puts`'s ARRAY-UNPACKING rule -- distinct from `sir_fmt`'s general case
# (`v.to_s`, which bracket-displays an Array, e.g. `"[1, 2, 3]"`), and from
# `print` below, which never unpacks. Real Ruby's `Kernel#puts` special-cases
# an Array argument: each element gets its OWN line, RECURSIVELY flattening
# nested arrays, and an EMPTY array prints nothing at all (not even a blank
# line) -- `puts [1, [2, 3], 4]` -> "1\n2\n3\n4\n"; `puts []` -> (nothing).
# A Hash argument is NOT unpacked (only Array), so this checks `Array`
# specifically. No depth cap is needed here (unlike the C backend's
# `_sir_puts_one`): a self-referential array would raise Ruby's own
# `SystemStackError` on infinite recursion -- safe, since this runs under a
# real Ruby VM with its own stack-overflow protection, not raw C recursion.
def sir_puts_one(x)
  if x.is_a?(Array)
    x.each { |e| sir_puts_one(e) }
  else
    STDOUT.write(sir_fmt(x))
    STDOUT.write("\n")
  end
end

def sir_puts(*xs)
  if xs.empty?
    STDOUT.write("\n")
  else
    xs.each { |x| sir_puts_one(x) }
  end
  nil
end

def sir_print(*xs)
  xs.each { |x| STDOUT.write(sir_fmt(x)) }
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
  when "print" then sir_print(*args)
  when "puts"  then sir_puts(*args)
  else
    STDERR.puts("sir: undefined builtin '#{name}'")
    exit(1)
  end
end

def sir_builtin_closure(name)
  ->(*args) { sir_builtin_dispatch(name, args) }
end
"####;
