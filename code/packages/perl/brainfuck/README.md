# CodingAdventures::Brainfuck (Perl)

A Brainfuck interpreter and optimising compiler — two-phase execution with
pre-computed jump targets.

## What Is Brainfuck?

Brainfuck (Urban Müller, 1993) is a Turing-complete language with 8 commands.
It models a computer as a tape of 30,000 byte cells and a data pointer.

| Command | C equivalent    | Description                              |
|---------|-----------------|------------------------------------------|
| `>`     | `++ptr`         | Move data pointer right                  |
| `<`     | `--ptr`         | Move data pointer left                   |
| `+`     | `(*ptr)++`      | Increment current cell (wraps 255→0)     |
| `-`     | `(*ptr)--`      | Decrement current cell (wraps 0→255)     |
| `.`     | `putchar(*ptr)` | Output cell as ASCII                     |
| `,`     | `*ptr=getchar()`| Read input byte (0 on EOF)               |
| `[`     | `while(*ptr){`  | Jump past `]` if cell == 0               |
| `]`     | `}`             | Jump back to `[` if cell != 0            |

Any other character is a comment.

## Usage

```perl
use CodingAdventures::Brainfuck qw(interpret validate compile_to_opcodes run_opcodes);

# One call
my ($out, $err) = interpret("+++++++++[>++++++++<-]>.", "");
print $out;   # "H"  (9 * 8 = 72)

# Cat program
my ($cat_out) = interpret(",[.,]", "hello");
# $cat_out == "hello"

# Two-phase
my ($ops, $err2) = compile_to_opcodes("[+]");
my $result = run_opcodes($ops, "");
```

## API

### `validate($program)` → ($ok, $err)
### `compile_to_opcodes($program)` → ($opcodes_aref, $err)
### `run_opcodes($opcodes_aref, $input)` → $output_string
### `interpret($program, $input)` → ($output, $err)

## License

MIT
