# x509_extension

Bounded, authority-free generic X.509 `Extension` decoding for Ruby. The
package validates the extension OID, canonical optional `critical` flag, and
opaque OCTET STRING value while sharing the caller's `der_asn1` limits.

Values and errors use private construction capabilities, immutable snapshots,
stable categories, and payload-redacted diagnostics.

## Dependencies

- coding_adventures_der_asn1
- coding_adventures_der_tlv

## Development

```bash
bash BUILD
# Windows
cmd /c BUILD_windows
```

The tests consume all 48 closed `x509-extension-v1` fixture cases.
