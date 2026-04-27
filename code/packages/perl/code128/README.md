# code128 (Perl)

Dependency-free Code 128 encoder that emits backend-neutral paint scenes.

## Usage

```perl
use CodingAdventures::Code128;

my $scene = CodingAdventures::Code128->draw_code128('HELLO-123');
```

This package stops at `PaintScene` so the same barcode logic can feed native
Paint VMs, Canvas, SVG, or future codecs.
