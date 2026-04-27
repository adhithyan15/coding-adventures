# itf (Perl)

Dependency-free ITF encoder that emits backend-neutral paint scenes.

## Usage

```perl
use CodingAdventures::Itf;

my $scene = CodingAdventures::Itf->draw_itf('123456');
```

This package stops at `PaintScene` so the same barcode logic can feed native
Paint VMs, Canvas, SVG, or future codecs.
