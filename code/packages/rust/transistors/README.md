# transistors

Transistor models and CMOS/TTL gate construction — the foundation of all digital hardware.

## What This Crate Does

This crate models transistors at the electrical level and builds logic gates from them.
While the `logic-gates` crate treats gates as abstract truth tables (input 0/1, output 0/1),
this crate shows what happens *inside* the gate: voltages, currents, power dissipation,
and propagation delays.

## Where It Fits in the Stack

```
logic-gates     <- abstract boolean logic (0 and 1)
transistors     <- THIS CRATE: electrical reality beneath the abstraction
silicon physics <- not modeled (take a physics course!)
```

The `logic-gates` crate builds on top of this one conceptually. This crate answers
the question: "How do you actually build an AND gate from physical components?"

## Modules

| Module       | Description                                              |
|-------------|----------------------------------------------------------|
| `types`      | Enums, parameter structs, result types                   |
| `mosfet`     | NMOS and PMOS field-effect transistors                   |
| `bjt`        | NPN and PNP bipolar junction transistors                 |
| `cmos_gates` | NOT, NAND, NOR, AND, OR, XOR gates from MOSFET pairs    |
| `ttl_gates`  | Historical TTL NAND and RTL inverter from BJTs           |
| `amplifier`  | Common-source and common-emitter amplifier analysis      |
| `analysis`   | Noise margins, power, timing, CMOS vs TTL comparison    |

## Usage Examples

```rust
use transistors::mosfet::NMOS;
use transistors::cmos_gates::CMOSInverter;

// Create a default NMOS transistor (180nm process)
let nmos = NMOS::new(None);
assert!(nmos.is_conducting(1.0));   // Vgs=1.0V > Vth=0.4V -> ON
assert!(!nmos.is_conducting(0.2));  // Vgs=0.2V < Vth=0.4V -> OFF

// Build a CMOS inverter and evaluate it
let inv = CMOSInverter::new(None, None, None);
assert_eq!(inv.evaluate_digital(0).unwrap(), 1);  // NOT 0 = 1
assert_eq!(inv.evaluate_digital(1).unwrap(), 0);  // NOT 1 = 0

// Get full electrical details
let result = inv.evaluate(0.0);  // Input = 0V
println!("Output voltage: {} V", result.voltage);
println!("Power: {} W", result.power_dissipation);
println!("Delay: {} s", result.propagation_delay);
```

## Running Tests

```bash
cargo test -p transistors -- --nocapture
```

## Port

This is a Rust port of the Python `transistors` package, with identical logic and
test coverage. The Rust version uses the type system to enforce correctness at
compile time (e.g., `u8` for digital values, `Result` for validation).
