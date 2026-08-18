# Changelog

## Unreleased

- Added the crate, as the pure half of `VLT-PM44-cli-password-generate.md`.

- Added `CharacterClassesV1` and the four class tables — lowercase (26),
  uppercase (26), digits (10), and 27 symbols. The symbol set is the printable
  US-ASCII punctuation minus `"`, `'`, `` ` ``, `\`, and `/`, which are the
  characters most likely to be mangled by a shell, a CSV import, a JSON blob,
  or a hand-written SQL string on the way to the site the password is for.

- Added `AMBIGUOUS_CHARACTERS` (`0`, `1`, `I`, `O`, `l`, `|`) for
  `--exclude-ambiguous`. Each belongs to exactly one class, so the exclusion can
  never empty a class the caller selected; a test asserts that property rather
  than trusting the table.

- Added `PasswordPolicyV1`, a validated policy that cannot exist unless it is
  1–128 characters long, selects at least one class, and reaches the entropy
  floor. Validation runs shape, then alphabet, then strength, so a request that
  is wrong twice reports the most basic problem.

- Added `MIN_PASSWORD_ENTROPY_BITS = 80` and `meets_minimum_entropy`, which
  decides the floor by the exact integer comparison `alphabet^length >= 2^80`
  rather than by comparing `length * log2(n)` against `80.0`. Two of the eight
  documented policy rows land within 0.2 bits of the line, so a floating-point
  check would put rounding in charge of a security boundary and could answer
  differently on different platforms. Tests assert acceptance and refusal on
  both sides of all eight rows.

- Added `generate_password`, which consumes exactly
  `PasswordPolicyV1::required_entropy_bytes` caller-supplied bytes as 8-byte
  big-endian words and returns a wipe-on-drop `Zeroizing<String>`:
  - Selection is **exactly** uniform. A word below `floor(2^64 / n) * n`
    selects `alphabet[word mod n]`; a word at or above it is discarded and the
    next word is read. The bound is computed in `u128` because the span being
    divided is `2^64`, which a `u64` cannot hold.
  - `SPARE_ENTROPY_WORDS = 8` covers discards. Exhausting the reserve is
    `EntropyExhausted`, never a fallback to a biased `word mod n` — otherwise
    the one branch nobody ever exercises would be the branch that silently
    weakens the output.
  - The string is allocated at its exact final capacity before the first push,
    so growth can never strand an unwiped plaintext prefix on the heap.
  - A reserve of the wrong size is refused in **both** directions; an over-long
    buffer means the caller and this crate disagree about the reservation, which
    is not a disagreement to paper over.

- Deliberately sources no randomness: no `rand`, no thread-local generator, no
  seeding, and no entry point that produces a password without being handed
  bytes. The only dependency is `coding_adventures_zeroize`, and
  `required_capabilities.json` is empty.

- Deliberately does not force class inclusion. Every character is drawn
  independently from the whole alphabet, because constraining a uniform
  sampler's output removes entropy and would make the strength claim the floor
  is checked against untrue.

- Added 19 unit tests plus a doc test, covering the class tables and their
  disjointness, alphabet composition for twelve class/ambiguity combinations,
  both sides of every entropy-floor row, degenerate alphabets, validation
  ordering, determinism, alphabet containment, reserve-size mismatch in both
  directions, the acceptance bound's maximality, discard-versus-reduce
  behaviour proven against what a biased sampler would have emitted, reserve
  exhaustion, spare-word absorption at exactly its limit, and a distribution
  sanity check over 25,600 draws that also rejects short-period cycles and
  repeated outputs.
