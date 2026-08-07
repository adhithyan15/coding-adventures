# chief-of-staff-skill-package

This package closes the trusted boundary between a zero-code Level 1
`SKILL.md` and Chief's sealed agent catalog. A build produces exactly two
signed content files: the original `SKILL.md` and its canonical generated
`manifest.json`. Authentication metadata is then added by the shared host
package signer.

Level 1 is intentionally a distinct package runtime from deny-all Deno. The
trusted Rust skill runtime already owns provider-neutral LLM execution, so a
generated Deno shim would introduce a second, undeclared LLM API. Discovery
still sees one `VerifiedAgentPackage` contract for both layouts.

Loading never re-reads package files. It parses the `SKILL.md` bytes retained
by verification and requires its newly derived canonical manifest to match the
authenticated `manifest.json` byte-for-byte. This prevents a package from
signing different instructions and policy metadata.
