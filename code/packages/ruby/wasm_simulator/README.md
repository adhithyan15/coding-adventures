# WASM Simulator

Simulates a subset of WebAssembly: i32.const, i32.add, i32.sub, local.get,
local.set, end. Part of the **coding-adventures** computing stack.

## Usage

```ruby
require "coding_adventures_wasm_simulator"

sim = CodingAdventures::WasmSimulator::WasmSimulator.new(num_locals: 4)
program = CodingAdventures::WasmSimulator.assemble_wasm([
  CodingAdventures::WasmSimulator.encode_i32_const(1),
  CodingAdventures::WasmSimulator.encode_i32_const(2),
  CodingAdventures::WasmSimulator.encode_i32_add,
  CodingAdventures::WasmSimulator.encode_local_set(0),
  CodingAdventures::WasmSimulator.encode_end
])
traces = sim.run(program)
puts sim.locals[0] # => 3
```
