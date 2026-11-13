---
category: Security boundaries
---

# A width parameter that sets an output size is an amplification factor and needs a cap before the shift

**What went wrong.** `logic_gates::decoder` allocated `1usize << n` output lines for an input of `n` bits and `demux` is built on it. The caller's `n` therefore multiplies the work and memory: 17 to 40 inputs mean a 128 KiB to 1 TiB allocation and 2^n loop turns, and 64 inputs shifted by the full width, which panics in debug and WRAPS to `1 << 0` in release, returning one output line for sixty-four inputs. A hostile or merely buggy caller could hang the process, abort it, or get a wrong answer silently.

**Fix.** A public cap (`MAX_DECODER_WIDTH = 16`), checked with an `assert!` before the shift and before any allocation, with a test at the cap, one past it and at 64, and examples in the Warrant file that fail if the cap is removed.

**Do differently.** Whenever an input length or width feeds `1 << n`, `n * n`, `vec![_; n]` or a recursion depth, ask what the output size is as a function of the caller's value and bound it explicitly; check the shift width against the type's bit count. Test the exact boundary, the first value past it, and a value at or above the shifted type's width in both profiles.
