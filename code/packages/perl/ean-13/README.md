# ean-13 (Perl)

Dependency-free EAN-13 encoder that emits backend-neutral paint scenes.

## Usage

```perl
use CodingAdventures::Ean13;

my $scene = CodingAdventures::Ean13->draw_ean_13('400638133393');
```

This package stops at `PaintScene` so the same barcode logic can feed native
Paint VMs, Canvas, SVG, or future codecs.
