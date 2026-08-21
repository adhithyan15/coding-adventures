# coding_adventures_image_codec_png

A pure Dart implementation of the bounded IC18 portable PNG profile. It
encodes deterministic non-interlaced RGBA8 PNGs and decodes the profile's
8-bit grayscale, truecolour, grayscale-alpha, and RGBA inputs.

The codec delegates CRC-32 and raw RFC 1951 compression to the repository's
`coding_adventures_zip` package. It owns only PNG framing, RFC 1950 zlib
wrapping, filtering, colour expansion, and the IC18 validation order.

## Public API

```dart
import 'package:coding_adventures_image_codec_png/coding_adventures_image_codec_png.dart';
import 'package:coding_adventures_pixel_container/coding_adventures_pixel_container.dart';

final pixels = PixelContainer(2, 2)..fill(255, 0, 0, 255);
final encoded = encodePng(pixels);
final decoded = decodePng(encoded);

final codec = PngCodec(maxPixels: 1024);
assert(codec.mimeType == 'image/png');
assert(codec.decode(codec.encode(pixels)) == pixels);
```

`maxDimension` is 16,384 and `defaultMaxPixels` is 33,554,432. Callers may
lower, but never raise, the pixel ceiling. `PngError.code` and its message are
the same payload-blind identifier from the immutable 29-code IC18 taxonomy.

## Portable contract

The public API consumes the shared `image-codec-png-v1` corpus: 85 cases
covering decoding, typed failures, deterministic encoding, filters, colour
types, transparency, chunk order, CRC and Adler checks, exact DEFLATE
consumption, allocation limits, and APNG rejection precedence. Encoder output
is also decoded with the independent test-only `image` package.

Dart's `PixelContainer` constructor prevents fractional dimensions and
wrong-length RGBA buffers. The language-neutral fixture adapter therefore maps
those unrepresentable JSON boundary cases to the same `PngError` codes before
typed construction. `encodePng` still revalidates dimensions, pixel count, and
buffer length; focused tests use a malformed subclass to keep that boundary
load-bearing.

## Authority and resource bounds

Production code is pure and in-memory. It imports only `dart:typed_data`, the
repository PixelContainer, and the repository ZIP substrate. It has no
filesystem, network, process, environment, clock, entropy, console, FFI,
native, or credential authority. Fixture file access, the platform zlib
decoder, and the foreign PNG decoder are test-only.

Dimensions and caller limits are checked before derived allocation. Raw
inflate is capped at the exact filtered byte count promised by IHDR, must
consume the entire DEFLATE payload, and is followed by Adler and filter
validation before the RGBA output is allocated.

## Development

```text
dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=90
```
