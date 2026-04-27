# coding_adventures_zeroize

Secure memory zeroization for Rust, implemented from scratch — no
external dependencies.

## What It Does

Provides a primitive and a trait for **observably wiping secret bytes
from RAM**:

1. `zeroize_bytes(slice)` — volatile-stores `0` to every byte of `slice`,
   then issues a `compiler_fence(SeqCst)`. The compiler is forbidden
   from eliding the stores or reordering them past the fence.
2. `trait Zeroize` — implemented for `[u8]`, `[u8; N]`, `Vec<u8>`,
   `String`, `Option<T: Zeroize>`, and every fixed-width integer.
3. `Zeroizing<T>` — RAII wrapper whose `Drop` calls `.zeroize()` on the
   inner value. Use this for any local that holds a secret.

## Why A Plain Assignment Isn't Enough

If you write:

```rust
let mut key = [0u8; 32];
fill_key(&mut key);
use_key(&key);
key = [0; 32];  // "clear" the key
```

the compiler sees that `key` is never read after the final assignment,
so the final assignment is *dead-store-eliminated*. Your secret is
still sitting in the stack frame (or register spill slot, or heap
allocation) until the memory is reused for something else.

`zeroize_bytes` uses `core::ptr::write_volatile`, which the compiler is
required to emit even if the write appears dead, plus a
`compiler_fence(SeqCst)` so the writes can't be reordered past the end
of the buffer's lifetime.

## What It Is Not

- **Not a paging/swap defence.** If the page has already been swapped
  to disk, scrubbing the in-RAM copy doesn't help. Use `mlock` at the
  OS level.
- **Not a cold-boot defence.** Zeroization runs in software; capacitor
  decay and DRAM retention happen regardless.
- **Not a live-attacker defence.** Anyone who can read your memory
  *while the secret is alive* can read it. Zeroization shrinks the
  window — it doesn't eliminate it.
- **Not a constant-time compare.** Use the separate constant-time
  compare crate for tag equality.

## Usage

```rust
use coding_adventures_zeroize::{Zeroize, Zeroizing, zeroize_bytes};

// Primitive
let mut buf = [0xAAu8; 32];
zeroize_bytes(&mut buf);
assert_eq!(buf, [0u8; 32]);

// Trait — works on slices, arrays, Vec, String, ints, Option
let mut key: [u8; 32] = random_key();
key.zeroize();

// RAII — wipes on drop, including during panic unwind
{
    let secret = Zeroizing::new(load_master_key());
    // ... use *secret ...
} // <- key bytes are wiped here
```

## How It Fits

Part of the D18 Chief-of-Staff Vault's crypto stack. The vault master
key, channel master keys, and every short-lived lease handed back to
an agent live inside a `Zeroizing<…>` so that every exit path — normal
return, early return, panic unwind — wipes the material before the
stack slot or heap allocation is reused.

Self-contained: no runtime dependencies.

## Implementation Notes

- `write_volatile` in a loop, followed by `compiler_fence(SeqCst)`.
  `compiler_fence` is intentionally chosen over the hardware `fence`:
  the defence is against the compiler, not other CPU cores.
- `Vec<u8>::zeroize` scrubs the entire `capacity()`, not just the live
  prefix, because earlier growth may have left secret bytes in the
  unused tail.
- `String::zeroize` delegates to `Vec<u8>::zeroize` through
  `as_mut_vec`; an all-zero byte sequence is valid UTF-8, so the
  invariant is never broken.
- `Zeroizing<T>` does **not** implement `Debug`, `Display`, or
  `Clone`. Printing or duplicating secret material is exactly the
  mistake this wrapper is designed to prevent.
- `Zeroizing::into_inner` is an explicit escape hatch for when the
  secret must outlive the wrapper; it skips the wipe.
