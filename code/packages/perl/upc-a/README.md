# upc-a (Perl)

Dependency-free UPC-A encoder that emits backend-neutral paint scenes.

## Usage

```perl
use CodingAdventures::UpcA;

my $scene = CodingAdventures::UpcA->draw_upc_a('03600029145');
```

This package stops at `PaintScene` so the same barcode logic can feed native
Paint VMs, Canvas, SVG, or future codecs.
