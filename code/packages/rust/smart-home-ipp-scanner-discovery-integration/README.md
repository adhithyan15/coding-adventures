# smart-home-ipp-scanner-discovery-integration

This package discovers IPP Scan Service scanners through the standard
`_scan._sub._ipp._tcp.local` DNS-SD browse name. It reuses the production Smart
Home mDNS scanner and validates the scanner resource path, TXT version,
optional canonical UUID, model, location, authentication requirement, TLS
version, document formats, automatic document feeder, transparency adaptor,
and push-destination schemes before recording verified D23 candidates.

Discovery does not open the IPP TCP endpoint. It performs no IPP request,
scanner-status read, credential input, scan submission, document retrieval,
push-destination access, public-endpoint access, or long-lived browse. Those
operations require separate supervised transport, secret-custody,
data-governance, destination-policy, and operation-policy owners.

Official references:

- <https://ftp.pwg.org/pub/pwg/candidates/cs-ippscan10-20140918-5100.17.pdf>
- <https://www.rfc-editor.org/rfc/rfc6763.html>
- <https://www.rfc-editor.org/rfc/rfc8011.html>

```sh
cargo run -p smart-home-ipp-scanner-discovery-integration -- discover
```
