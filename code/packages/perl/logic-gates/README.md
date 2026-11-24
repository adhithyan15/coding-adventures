# logic-gates

Pure-Perl logic-gates implementation

## Development

```bash
# Run tests
bash BUILD
```

## Calling convention

All exported functions are plain functions. Class-method style (`CodingAdventures::LogicGates->AND(1, 0)`) is not supported: guessing an invocant can silently drop an argument, so a leading package-name argument is treated as data and rejected. Bits must be the integers or strings `0`/`1`; anything else dies.
