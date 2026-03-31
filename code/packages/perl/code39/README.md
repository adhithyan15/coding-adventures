# code39 (Perl)

Code 39 barcode encoder — normalize, encode, expand runs, render SVG.

## Usage

```perl
use CodingAdventures::Code39;

my $scene = CodingAdventures::Code39->draw_code39('HELLO123');
print $scene->{svg};
```

## Dependencies

None — self-contained.
