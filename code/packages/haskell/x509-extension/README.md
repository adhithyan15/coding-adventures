# x509-extension

Bounded, authority-free generic X.509 `Extension` decoding for Haskell. The
package validates the extension OID, canonical optional `critical` flag, and
opaque OCTET STRING value while sharing the caller's `der-asn1` work limits.

The API exposes immutable values and stable payload-redacted errors. Extension
contents remain opaque; certificate policy, signatures, trust, revocation, and
transport are deliberately out of scope.

## Dependencies

- der-asn1
- der-tlv

## Development

```bash
bash BUILD
# Windows
cmd /c BUILD_windows
```

The test suite consumes all 48 closed `x509-extension-v1` fixture cases.
