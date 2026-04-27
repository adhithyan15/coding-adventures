# codabar (Perl)

Dependency-free Codabar encoder that emits backend-neutral paint scenes.

## Usage

```perl
use CodingAdventures::Codabar;

my $scene = CodingAdventures::Codabar->draw_codabar('A1234B');
```

This package stops at `PaintScene` so the same barcode logic can feed native
Paint VMs, Canvas, SVG, or future codecs.
