# smart-home-ipp-printer-discovery-integration

This package discovers IPP and IPP Everywhere printers through the standard
`_ipp._tcp.local` DNS-SD service. It reuses the production Smart Home mDNS
scanner, validates the resource path, TXT version, optional printer UUID,
model, location, authentication requirement, TLS version, document formats,
and advertised color and duplex capabilities, then records verified D23
discovery candidates after authorization.

For an explicitly selected credential-free discovery candidate, the package
can also perform one authorized, bounded IPP/1.1
`Get-Printer-Attributes` request over local HTTP. It strictly correlates the
request id and fixed response allowlist, then normalizes printer identity,
state, reasons, job acceptance, queue count, and uptime into one confirmed D23
diagnostic entity.

The status runtime accepts no credential and exposes no print submission, job
query, job document, queue mutation, arbitrary attribute selection, public
endpoint, DNS resolution, TLS exception, redirect, subscription, or long-lived
connection. Credentialed and IPPS-only endpoints require a separately
supervised authentication and TLS owner.

Official references:

- <https://ftp.pwg.org/pub/pwg/candidates/cs-ippeve11-20200515-5100.14.pdf>
- <https://www.rfc-editor.org/rfc/rfc6763.html>
- <https://www.rfc-editor.org/rfc/rfc8011.html>
- <https://www.rfc-editor.org/rfc/rfc8010.html>

```sh
cargo run -p smart-home-ipp-printer-discovery-integration -- discover
```
