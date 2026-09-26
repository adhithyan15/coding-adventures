# Use governing capability manifest keys exactly

The Kotlin DER ASN.1 package copied the parity fixture convention and wrote
`schema_version`, but the repository's capability manifest schema requires
`version` and rejects additional properties. Validate new metadata against its
governing JSON Schema (and a current same-language package) before considering
a lane complete.
