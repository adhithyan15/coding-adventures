# CodingAdventures.PolynomialNative

Elixir NIF (Native Implemented Function) wrapping the Rust `polynomial` crate
via the zero-dependency `erl-nif-bridge`. Provides fast polynomial arithmetic
over `f64` with no Rustler or `erl_nif.h` required.

## Where it fits

```
CodingAdventures.PolynomialNative  (this package — Elixir NIF)
         │
         └── erl-nif-bridge (Rust)  ──── Erlang/OTP NIF C API
         └── polynomial (Rust)      ──── core arithmetic
```

## Usage

```elixir
alias CodingAdventures.PolynomialNative, as: P

# Polynomials are lists of floats, index = degree
# [3.0, 0.0, 1.0] = 3 + 0·x + 1·x²
P.add([1.0, 2.0], [3.0, 4.0])        # => [4.0, 6.0]
P.multiply([1.0, 2.0], [3.0, 4.0])   # => [3.0, 10.0, 8.0]
P.evaluate([3.0, 0.0, 1.0], 2.0)     # => 7.0
{q, r} = P.divmod([5.0,1.0,3.0,2.0], [2.0,1.0])
P.degree([3.0, 0.0, 2.0])            # => 2
```

## Building

```bash
# Build the Rust NIF shared library and run tests
bash BUILD
```

Or manually:

```bash
cd native/polynomial_native && cargo build --release
mix deps.get && mix compile && mix test
```

## Design notes

- The NIF shared library is compiled separately from Mix via `cargo build --release`.
- Division-by-zero panics in Rust are caught with `catch_unwind` and returned
  as `badarg` to Elixir — no BEAM crash.
- The module atom name in the NIF entry (`"polynomial_native"`) must match the
  path passed to `:erlang.load_nif/2`.
