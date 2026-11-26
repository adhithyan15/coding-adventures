# transistors

Pure-Perl transistors implementation

## Development

```bash
# Run tests
bash BUILD
```

## Calling convention

All exported functions are plain functions. Class-method style (`CodingAdventures::Transistors->cmos_and(1, 0)`) is not supported: guessing an invocant can silently drop an argument, so a leading package-name argument is treated as data and rejected. Bits must be the integers or strings `0`/`1`; anything else dies.

## Errors

Every rejection throws a `CodingAdventures::Transistors::Error` object. Match on
its stable `code`, never on the message text:

| code             | meaning                                                  |
|------------------|----------------------------------------------------------|
| `INVALID_BIT`    | a digital argument is not the bit 0 or 1                 |
| `INVALID_ARITY`  | wrong number of arguments                                |
| `INVALID_NUMBER` | an analog argument (vgs, vth, ib, beta) is not finite    |

```perl
use CodingAdventures::Transistors qw(cmos_and is_invalid_bit);
eval { cmos_and("x", 1); 1 } or do {
    my $e = $@;
    print $e->code, " ", $e->param, " ", $e->value_description, "\n";  # INVALID_BIT a 'x'
    print "bad bit\n" if is_invalid_bit($e);
};
```

The object stringifies to the human message, so `die`/`$@` output is unchanged.
`value_description` is a bounded rendering, never the raw value.
`is_invalid_bit` is true only for an exact-class Error with code `INVALID_BIT`.
