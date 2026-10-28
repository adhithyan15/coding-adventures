---
category: Compiler / VM / language pipeline
---

# A register cap that counts distinct names must also count positional parameter slots

**What went wrong.** The vm-core fix that made the 65,536-register frame
limit a load-time error first counted a function's registers as the
*distinct names* among its parameters and instruction `dest`s. It also
clamped the pre-allocated register file to the limit. But parameters are
positional: the VM writes argument `i` straight into `registers[i]`,
whatever the name. A function with 65,537 parameters all named `a` has one
distinct name, so it passed the check. Its frame was clamped to 65,536
slots, and writing the 65,537th argument panicked with index out of bounds,
through `execute` and through `call`. Security review caught it. On main
this had not panicked, because the frame was sized to `params.len()`.

The same mismatch was already on main in a quieter form. A new register was
numbered `name_to_reg.len()`. With duplicate parameter names the map is
shorter than the parameter slots, so in `f(a, a)` the first local got slot
1 and overwrote the second argument.

**Fix.** Count `params.len()` plus the distinct `dest` names that are not
parameters. Number new registers from a separate `next_reg` counter that
starts at `params.len()`. `VMFrame::for_function` returns the typed error
when `params.len()` exceeds the limit, so no argument write can go out of
bounds even if the load-time check is bypassed.

**Next time.** When a limit counts "registers", find every way a slot gets
allocated: by name (a map) *and* by position (parameters, indexed writes).
Test with duplicate names. Make the counter that hands out slots the same
quantity the limit check counts, never a container's `len()`, which can
diverge. Clamping a buffer is only safe when every index written into it
is bounded by the same clamp.
