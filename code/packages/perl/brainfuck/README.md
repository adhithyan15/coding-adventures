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

## Package Structure

| File          | Purpose                                            |
|---------------|----------------------------------------------------|
| `Brainfuck.pm`| Interpreter: `validate`, `compile_to_opcodes`, `run_opcodes`, `interpret` |
| `Lexer.pm`    | Grammar-driven tokenizer (`tokenize`)               |
| `Parser.pm`   | Grammar-driven parser (`parse`), returns AST        |

## Usage

### Lexer

```perl
use CodingAdventures::Brainfuck::Lexer qw(tokenize);

my ($tokens, $err) = tokenize("++[>+<-].");
for my $tok (@$tokens) {
    printf "%s %s at %d:%d\n", $tok->{type}, $tok->{value}, $tok->{line}, $tok->{column};
}
# COMMAND + at 1:1
# COMMAND + at 1:2
# LOOP_START [ at 1:3
# ...
```

### Parser

```perl
use CodingAdventures::Brainfuck::Parser qw(parse);

my ($ast, $err) = parse("++[>+<-].");
# $ast = {
#   type     => "program",
#   children => [
#     { type => "instruction", children => [{ type => "command", value => "+" }] },
#     { type => "instruction", children => [{ type => "command", value => "+" }] },
#     { type => "loop",        children => [...] },
#     { type => "instruction", children => [{ type => "command", value => "." }] },
#   ]
# }
print $ast->{type};  # "program"
```

### Interpreter

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

### `tokenize($source)` → ($tokens_aref, $err)

Tokenize Brainfuck source. Returns an array ref of token hashrefs (`{type, value, line, column}`). Comment characters are skipped. Returns `(undef, $message)` on error.

### `parse($source)` → ($ast_hashref, $err)

Parse Brainfuck source into an AST. Returns a root hashref (`{type => "program", children => [...]}`). Returns `(undef, $message)` with line/column info on unmatched bracket.

### `validate($program)` → ($ok, $err)
### `compile_to_opcodes($program)` → ($opcodes_aref, $err)
### `run_opcodes($opcodes_aref, $input)` → $output_string
### `interpret($program, $input)` → ($output, $err)

## Where It Fits in the Stack

This package now spans Layers 2–5 of the coding-adventures computing stack:

```
Layer 5: Interpreter    [brainfuck]        ← YOU ARE HERE
Layer 4: Compiler       [bytecode-compiler]
Layer 3: Parser         [parser]           ← also implemented here
Layer 2: Lexer          [lexer]            ← also implemented here
Layer 1: Grammar Tools  [grammar-tools]
```

## License

MIT
