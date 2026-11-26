## 0.1.1

- Harden validation: reject `tagLength > 0xffffffff` with the
  reference's own `TagLengthTooLarge` variant. The reference takes
  `tag_length` as `u32` so the case is inexpressible there, but
  Dart ints are 64-bit — without the cap, H0 would silently
  truncate the length field and H' would attempt a t-byte
  allocation. Found by the Dart argon2d security review; the
  argon2d/argon2i lanes shipped with the cap.
