# frozen_string_literal: true

# ==========================================================================
# TrapError --- An Unrecoverable WASM Runtime Error
# ==========================================================================
#
# In WebAssembly, a "trap" is a fatal runtime error. When execution hits
# an illegal operation --- dividing by zero, accessing memory out of bounds,
# executing the `unreachable` instruction --- a trap occurs and immediately
# halts execution. There is no exception handling within WASM 1.0; the trap
# propagates to the host environment.
#
# Common causes of traps:
#
#   - Out-of-bounds memory access
#   - Out-of-bounds table access (call_indirect with bad index)
#   - Division by zero (integer division only; float yields NaN/Infinity)
#   - Integer overflow in division (e.g., i32.div_s(-2147483648, -1))
#   - Unreachable instruction executed
#   - Type mismatch in call_indirect
#
# We model traps as a custom exception class so that host code can distinguish
# them from ordinary Ruby errors via `rescue TrapError`.
#
#   ┌─────────────────────────────────────────────────────────────────┐
#   │                   Trap Error Flow                               │
#   │                                                                 │
#   │   WASM Module              Host Environment                     │
#   │  ┌──────────────┐        ┌──────────────────┐                   │
#   │  │  i32.div_s   │─trap!─>│  begin ... end    │                  │
#   │  │  (n / 0)     │        │  rescue TrapError │                  │
#   │  └──────────────┘        │    # handle it    │                  │
#   │                          └──────────────────┘                   │
#   └─────────────────────────────────────────────────────────────────┘
# ==========================================================================

module CodingAdventures
  module WasmExecution
    # TrapError --- an unrecoverable WASM runtime error (a "trap").
    #
    # Raised when the execution engine encounters an illegal operation.
    # Host code can rescue this to handle WASM traps gracefully.
    #
    #   begin
    #     engine.call_function(0, [i32(5)])
    #   rescue CodingAdventures::WasmExecution::TrapError => e
    #     puts "WASM trapped: #{e.message}"
    #   end
    #
    class TrapError < RuntimeError; end
  end
end
