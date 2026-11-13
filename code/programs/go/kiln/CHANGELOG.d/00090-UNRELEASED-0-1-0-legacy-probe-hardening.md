## [0.1.0] - UNRELEASED

### Fixed

- Security review of 7c-3 (KILN03 section 13):
  - the probe checkout refusal now fails closed: `Probe` needs a root, a path
    that cannot be related to it counts as inside, macOS and Windows compare
    case-folded, and a `HOME` that is not absolute and clean is treated as unset;
  - a probe may only run a tool its entry pins, with flag-shaped arguments and
    no interpreter eval flags; the checked real path is the one executed;
  - a non-zero probe exit is a mismatch (output kept as `detail`); each probe
    runs in its own process group killed on timeout; all probes share a 120 s
    limit (`probe-timeout`); cleanup errors are reported;
  - env: last duplicate wins, case-insensitive on Windows, more secret-like
    name markers and credential-shaped values refused;
  - policy paths refuse `:` and `;`; `ephemeral` globs need two literal
    segments; every policy file is decoded case-exactly with duplicate keys
    refused.
