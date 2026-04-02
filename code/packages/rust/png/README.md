# png

Zero-dependency PNG file format encoder.  Uses our `deflate` crate for
compression and implements CRC-32 for chunk checksums.

Encodes raw RGBA pixel data into valid PNG files.  No external libraries.
