# smart-home-ipp-printer-discovery-integration

This package discovers IPP and IPP Everywhere printers through the standard
`_ipp._tcp.local` DNS-SD service. It reuses the production Smart Home mDNS
scanner, validates the resource path, TXT version, optional printer UUID,
model, location, authentication requirement, TLS version, document formats,
and advertised color and duplex capabilities, then records verified D23
discovery candidates after authorization.

Discovery does not open the IPP TCP endpoint. It performs no IPP request,
printer-status read, credential input, print submission, queue mutation,
public-endpoint access, or long-lived browse. Those operations require
separate supervised transport, secret-custody, data-governance, and operation
policy owners.

Official references:

- <https://ftp.pwg.org/pub/pwg/candidates/cs-ippeve11-20200515-5100.14.pdf>
- <https://www.rfc-editor.org/rfc/rfc6763.html>
- <https://www.rfc-editor.org/rfc/rfc8011.html>

```sh
cargo run -p smart-home-ipp-printer-discovery-integration -- discover
```
