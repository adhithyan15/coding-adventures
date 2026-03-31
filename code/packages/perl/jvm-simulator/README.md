# CodingAdventures::JvmSimulator

A Pure Perl simulator for the Java Virtual Machine (JVM) bytecode (`.class` file instructions).

## Usage

```perl
use CodingAdventures::JvmSimulator;

my $sim = CodingAdventures::JvmSimulator->new();

# Assemble: x = 1 + 2
my $code = CodingAdventures::JvmSimulator::assemble([
    CodingAdventures::JvmSimulator::encode_iconst(1),
    CodingAdventures::JvmSimulator::encode_iconst(2),
    [CodingAdventures::JvmSimulator::IADD],
    CodingAdventures::JvmSimulator::encode_istore(0),
    [CodingAdventures::JvmSimulator::RETURN],
]);

$sim->load($code);
my $traces = $sim->run();
print $sim->{locals}[0];  # 3
```

## Supported Opcodes

| Opcode      | Hex  | Description                            |
|-------------|------|----------------------------------------|
| iconst_0-5  | 0x03 | Push integer constant 0-5              |
| bipush      | 0x10 | Push signed byte (-128..127)           |
| sipush      | 0x11 | Push signed short (-32768..32767)      |
| ldc         | 0x12 | Load from constant pool                |
| iload_0-3   | 0x1A | Load int from local variable 0-3       |
| iload       | 0x15 | Load int from local variable N         |
| istore_0-3  | 0x3B | Store int to local variable 0-3        |
| istore      | 0x36 | Store int to local variable N          |
| iadd        | 0x60 | Integer add                            |
| isub        | 0x64 | Integer subtract                       |
| imul        | 0x68 | Integer multiply                       |
| idiv        | 0x6C | Integer divide (truncate toward zero)  |
| if_icmpeq   | 0x9F | Branch if integers equal               |
| if_icmpgt   | 0xA3 | Branch if int a > b                    |
| goto        | 0xA7 | Unconditional branch                   |
| ireturn     | 0xAC | Return integer value                   |
| return      | 0xB1 | Return void                            |

## Installation

```sh
cpanm --installdeps .
```

## Testing

```sh
prove -l -v t/
```
