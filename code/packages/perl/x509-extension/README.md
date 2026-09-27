# x509-extension

Bounded, authority-free generic X.509 `Extension` decoding for Perl. The
package validates the extension OID, canonical optional `critical` flag, and
opaque OCTET STRING value while sharing the caller's `der-asn1` limits.

Values are privately constructed and immutable through the public API. Errors
expose stable categories and extension-local offsets without rendering payloads.

## Dependencies

- coding-adventures-der-asn1

## Development

```bash
bash BUILD
# Windows
cmd /c BUILD_windows
```

The tests consume all 48 closed `x509-extension-v1` fixture cases.
