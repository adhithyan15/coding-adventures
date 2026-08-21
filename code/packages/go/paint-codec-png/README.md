# paint-codec-png

Pure Go PNG adapter for `pixel-container`.

The package preserves the paint-facing `PngCodec`, `Codec`, `Encode`,
`EncodePNG`, `Decode`, and `DecodePNG` APIs while delegating PNG behavior to
the repository's portable `image-codec-png` implementation. It therefore
shares IC18's deterministic RGBA8 encoder, stable `PngError` taxonomy,
16,384-pixel edge limit, 33,554,432-pixel product limit, bounded counted
inflate, exact IDAT consumption, and explicit APNG refusal.

The portable profile intentionally accepts a smaller PNG surface than Go's
standard library: non-RGBA8 profiles, Adam7 interlacing, and APNG are rejected
with stable typed errors. Production behavior is pure and in-memory; the Go
standard-library PNG decoder is used only in tests as an independent encoder
oracle.
