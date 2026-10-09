---
category: Security boundaries
---

# apt-get --print-uris quotes an MD5 sum on Ubuntu, so it cannot verify a cached archive

**What went wrong.** The first version of the verified .deb cache took each
archive's expected hash from `apt-get --print-uris`, and accepted only SHA256
or SHA512. Every unit test passed against a fake apt that printed SHA512. Run
against the real apt on Ubuntu 24.04, the same command prints

    'http://.../hello_2.10-3build1_amd64.deb' hello_2.10-3build1_amd64.deb 26006 MD5Sum:a8c3...

so every cached archive would have been rejected, and the cache would have
silently saved nothing.

**Fix.** The expected hash now comes from `apt-cache show`, which prints the
SHA256 and SHA512 fields of the same signed Packages index. The stanza is
matched by pool file name (percent-decoded from the URI; the pool name has no
epoch, unlike apt's local `%3a` name) and size. An end-to-end run against real
apt seeded the genuine archives and rejected a one-byte tamper.

**Next time.** A fake for an external tool encodes a belief about its output.
Check that belief once against the real tool before trusting tests built on
it, especially where the output decides what is trusted.
