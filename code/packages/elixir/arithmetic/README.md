# Arithmetic (Elixir)

Adders and ALU built entirely from logic gates. Every computation routes through AND, OR, NOT, and XOR gate functions from the `logic_gates` package.

## Where This Fits

```
Layer 4: Arithmetic (adders, ALU)  ← this package
    ↓ uses
Layer 3: Logic Gates (AND, OR, NOT, XOR)
Layer 2: Transistors
Layer 1: NAND gates
```

## Components

- **Half adder** — XOR for sum, AND for carry. Two gates total.
- **Full adder** — Two half adders + OR gate. Handles carry-in.
- **Ripple carry adder** — Chain of N full adders for N-bit addition.
- **ALU** — Arithmetic Logic Unit supporting 6 operations: ADD, SUB, AND, OR, XOR, NOT.

## Usage

```elixir
alias CodingAdventures.Arithmetic, as: Arith

# Half adder: {sum, carry}
Arith.half_adder(1, 1)  # => {0, 1}

# Full adder: {sum, carry}
Arith.full_adder(1, 1, 1)  # => {1, 1}

# 4-bit ripple carry adder (LSB first)
Arith.ripple_carry_adder([1, 0, 1, 0], [1, 1, 0, 0])  # 5 + 3 = 8

# ALU operations
result = Arith.alu_execute(:add, [1, 0, 1, 0], [1, 1, 0, 0])
result.value  # => [0, 0, 0, 1]  (8 in LSB-first)
result.carry  # => false
result.zero   # => false
```

## Running Tests

```bash
mix test
```
