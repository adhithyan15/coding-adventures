# logic-gates

Pure-Perl logic-gates implementation

## Development

```bash
# Run tests
bash BUILD
```

## Calling convention

All exported functions are plain functions. Class-method style (`CodingAdventures::LogicGates->AND(1, 0)`) is not supported: guessing an invocant can silently drop an argument, so a leading package-name argument is treated as data and rejected. Bits must be the integers or strings `0`/`1`; anything else dies.

## Errors

Every rejection throws a `CodingAdventures::LogicGates::Error` object. Match on
its stable `code`, never on the message text:

| code            | meaning                                         |
|-----------------|-------------------------------------------------|
| `INVALID_BIT`   | an argument is not the bit 0 or 1               |
| `INVALID_ARITY` | wrong number of arguments or inputs             |
| `INVALID_WIDTH` | counter width outside 1..65536 / not an integer |
| `INVALID_SHAPE` | container argument of the wrong type or length  |

```perl
use CodingAdventures::LogicGates qw(AND is_invalid_bit);
eval { AND("x", 1); 1 } or do {
    my $e = $@;
    print $e->code, " ", $e->param, " ", $e->value_description, "\n";  # INVALID_BIT a 'x'
    print "bad bit\n" if is_invalid_bit($e);
};
```

The object stringifies to the human message, so `die`/`$@` output is unchanged.
`value_description` is a bounded rendering, never the raw value.
`is_invalid_bit` is true only for an exact-class Error with code `INVALID_BIT`.
