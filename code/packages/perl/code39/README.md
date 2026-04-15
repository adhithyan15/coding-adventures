# code39 (Perl)

Dependency-free Code 39 encoder that emits backend-neutral paint scenes.

## Usage

```perl
use CodingAdventures::Code39;

my $scene = CodingAdventures::Code39->draw_code39('HELLO123');
```

This package stops at `PaintScene` so the same barcode logic can feed native
Paint VMs, Canvas, SVG, or future codecs.
