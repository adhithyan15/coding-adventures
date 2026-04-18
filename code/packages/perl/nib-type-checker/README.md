# nib-type-checker

`nib-type-checker` performs the semantic pass for Perl's Nib frontend.

It sits between parsing and IR lowering:

`Nib source -> nib-parser -> nib-type-checker -> nib-ir-compiler`

The checker keeps the result in the shared type-checker protocol shape so later
packages can reuse the same success and diagnostic contract.

## Example

```perl
use CodingAdventures::NibTypeChecker qw(check_source);

my $result = check_source(<<'NIB');
fn answer() -> u4 {
    return 7;
}
NIB

die $result->{errors}[0]{message} unless $result->{ok};
```

## Development

```bash
bash BUILD
```
