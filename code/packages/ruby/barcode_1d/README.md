# coding_adventures_barcode_1d

High-level 1D barcode pipeline for Ruby.

```ruby
require "coding_adventures_barcode_1d"

png = CodingAdventures::Barcode1D.render_png("HELLO-123", symbology: :code39)
```
