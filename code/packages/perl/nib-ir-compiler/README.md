# nib-ir-compiler

`nib-ir-compiler` lowers Perl's typed Nib AST into the shared compiler IR.

That makes it the bridge between the frontend and any backend:

`Nib source -> nib-type-checker -> nib-ir-compiler -> ir-to-*`

The emitted IR stays intentionally conservative so the existing backends can
lower it without Nib-specific special cases.

## Example

```perl
use CodingAdventures::NibIrCompiler qw(compile_source release_config);

my $result = compile_source('fn main() -> u4 { return 7; }', release_config());
print scalar @{ $result->{program}{instructions} };
```

## Development

```bash
bash BUILD
```
