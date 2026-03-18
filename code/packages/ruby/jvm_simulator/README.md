# JVM Simulator

Simulates JVM bytecode with real opcode values: iconst, bipush, ldc,
iload/istore, iadd/isub/imul/idiv, goto, if_icmpeq, if_icmpgt, ireturn,
return. Part of the **coding-adventures** computing stack.

## Usage

```ruby
require "coding_adventures_jvm_simulator"

sim = CodingAdventures::JvmSimulator::JVMSimulator.new
bytecode = CodingAdventures::JvmSimulator.assemble_jvm(
  [CodingAdventures::JvmSimulator::ICONST_1],
  [CodingAdventures::JvmSimulator::ICONST_2],
  [CodingAdventures::JvmSimulator::IADD],
  [CodingAdventures::JvmSimulator::ISTORE_0],
  [CodingAdventures::JvmSimulator::RETURN]
)
sim.load(bytecode)
traces = sim.run
puts sim.locals[0] # => 3
```
