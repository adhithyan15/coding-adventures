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

def sir_puts(*xs)
  if xs.empty?
    STDOUT.write("\n")
  else
    xs.each { |x| STDOUT.write(sir_fmt(x)); STDOUT.write("\n") }
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
