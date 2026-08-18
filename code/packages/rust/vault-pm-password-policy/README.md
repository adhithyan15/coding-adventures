# `coding_adventures_vault_pm_password_policy`

The pure half of `vault-pm password generate`, specified by
`code/specs/VLT-PM44-cli-password-generate.md`.

This crate validates a password policy, states exactly how many random bytes
that policy needs, and turns exactly those bytes into a password. It is the
answer to "how strong is this, and is it strong enough" — not to "where does
randomness come from".

## Where it fits

```text
vault-pm password generate            <- grammar, ceremony    (vault-pm-cli)
  └─ CliHost::fill_entropy            <- one reservation      (vault-pm-cli)
       └─ OsEntropy::fill             <- the trust boundary   (vault-pm-cli-host)
            └─ fill_random            <- getrandom/getentropy (csprng)
  └─ PasswordPolicyV1 / generate_password   <- this crate
  └─ ControllingTerminal::write_revealed_text  <- delivery    (vault-pm-cli-host)
```

## What it does

- **`PasswordPolicyV1::new(length, classes, exclude_ambiguous)`** validates a
  request. A value of this type is proof that the policy is within 1–128
  characters, selects at least one character class, and reaches the 80-bit
  entropy floor. There is no way to build one that does not.
- **`PasswordPolicyV1::required_entropy_bytes`** states the exact reservation:
  one 8-byte word per character plus eight spare words.
- **`generate_password(policy, entropy)`** maps exactly that many bytes to a
  wipe-on-drop `Zeroizing<String>`, deterministically.
- **`meets_minimum_entropy(alphabet_len, length)`** is the floor itself, exposed
  because the rule deserves to be checkable on its own.

## What it deliberately does not do

- **It sources no randomness.** No `rand`, no thread-local generator, no
  seeding. Nothing in this crate produces a password without being handed bytes
  first, which forces the caller to name its entropy source out loud — and the
  CLI's only source is the operating-system CSPRNG.
- **It does not touch a vault, a clock, a terminal, a file, or the network.**
  `required_capabilities.json` is empty and stays empty.
- **It does not force class inclusion.** Every character is drawn independently
  and uniformly from the whole alphabet. Constraining the output of a uniform
  sampler removes entropy, which would make the strength claim the floor is
  checked against untrue. VLT-PM44 §3.1 argues this at length, including the
  honest cost: a default 24-character draw contains no digit about 5.7% of the
  time.

## The two properties worth reading the source for

**The entropy floor is integer arithmetic, not floating point.** A policy is
accepted when `alphabet^length >= 2^80`, computed by multiplying and stopping
early. Deciding a security boundary by comparing `length * log2(n)` against
`80.0` would put rounding in charge of rows that land within 0.2 bits of the
line, and would let the answer differ between platforms.

**Selection is exactly uniform, not negligibly biased.** Randomness is consumed
as 8-byte big-endian words. A word below `floor(2^64 / n) * n` selects
`alphabet[word mod n]`; a word above it is discarded and the next is read. That
is why a reserve exists, and why exhausting the reserve is an error rather than
a fallback to `word mod n` — a fallback would make the branch nobody ever
exercises the one that silently weakens the output.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_password_policy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding_adventures_vault_pm_password_policy --no-deps
```
