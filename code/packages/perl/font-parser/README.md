# CodingAdventures::FontParser (Perl)

Metrics-only OpenType/TrueType font parser in pure Perl 5.26+ with minimal
dependencies (only core `Encode`). Part of the FNT series.

## Installation

```
cpanm .
```

Or manually:

```
perl Makefile.PL && make && make test && make install
```

## Quick start

```perl
use CodingAdventures::FontParser qw(load font_metrics glyph_id glyph_metrics kerning);

open my $fh, '<:raw', 'Inter-Regular.ttf' or die $!;
local $/;
my $data = <$fh>;
close $fh;

my $font = load($data);

my $m = font_metrics($font);
print $m->{units_per_em};     # 2048
print $m->{family_name};      # "Inter"
print $m->{ascender};         # 1984

my $gid_a = glyph_id($font, 0x0041);   # 'A'
my $gm    = glyph_metrics($font, $gid_a);
print $gm->{advance_width};   # e.g. 1401

print kerning($font, $gid_a, glyph_id($font, 0x0056));  # 0 (GPOS font)
```

## Error handling

```perl
use eval {
    my $font = load($bad_data);
};
if (my $err = $@) {
    print $err->{kind};     # "BufferTooShort" etc.
    print $err->{message};
}
```

## Development

```
prove -l -v t/
```

## License

MIT
