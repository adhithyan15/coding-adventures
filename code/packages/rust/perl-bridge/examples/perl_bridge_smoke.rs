// A Perl XS extension built only to be loaded into a real Perl by tests/smoke.pl.
//
// Why this exists
// ---------------
// perl-bridge is a Rust wrapper over a C shim that is compiled against the host
// Perl's headers.  A compiler cannot tell whether the shim and the Rust
// declarations agree about a type's width, whether a flag constant matches
// `sv.h`, or whether an XSUB leaves the argument stack in the state Perl expects;
// all of those compile.  This extension is a small XS module built on every part
// of the bridge, and tests/smoke.pl checks the results from the Perl side.
//
// Loading it
// ----------
// tests/lib/PerlBridgeSmoke.pm calls DynaLoader's `bootstrap`, which finds the
// staged library at auto/PerlBridgeSmoke/PerlBridgeSmoke.<dlext> and calls the
// exported `boot_PerlBridgeSmoke` below; that registers each XSUB with `newXS_unwind` (these XSUBs may `die`, so they are `extern "C-unwind"`).
//
// The error paths use `croak` (through `die`), which unwinds with `longjmp`
// across these Rust frames.  Every function that dies holds no value with a
// destructor at that point (only `&'static` data), so skipping its remaining
// frames leaks nothing and breaks no invariant.  Results are left on the Perl
// stack as new SVs without `sv_2mortal`, so each call leaks one SV: this is a
// test fixture, not a model for production code (see the XS consumers, which
// share the convention).

#![allow(non_snake_case, non_camel_case_types)]

use perl_bridge::*;
use std::ffi::{c_char, c_void};

/// Write result SV number `n` where Perl will look for it: `ST(n)`.
unsafe fn set_return(frame: &XsubFrame, n: i32, sv: *mut SV) {
    *frame.base.add((frame.ax + n) as usize) = sv;
}

/// Argument `n` of the current call.
unsafe fn arg(frame: &XsubFrame, n: i32) -> *mut SV {
    xs_arg_sv(frame.base, frame.ax, n)
}

/// Die unless the call has exactly `n` arguments.
unsafe fn need_args(frame: &XsubFrame, n: i32, name: &str) {
    if frame.items != n {
        die(&format!(
            "{name}: expected {n} arguments, got {}",
            frame.items
        ));
    }
}

// -- numbers ----------------------------------------------------------------

/// `add_iv($a, $b)`: integer addition through `sv_2iv` / `newSViv`.
unsafe extern "C-unwind" fn xs_add_iv(_cv: *mut CV) {
    let f = xsub_frame();
    need_args(&f, 2, "add_iv");
    let sum = sv_2iv(arg(&f, 0)).wrapping_add(sv_2iv(arg(&f, 1)));
    set_return(&f, 0, newSViv(sum));
    xsub_return(1, f.ax);
}

/// `add_nv($a, $b)`: float addition through `sv_2nv` / `newSVnv`.
unsafe extern "C-unwind" fn xs_add_nv(_cv: *mut CV) {
    let f = xsub_frame();
    need_args(&f, 2, "add_nv");
    let sum = sv_2nv(arg(&f, 0)) + sv_2nv(arg(&f, 1));
    set_return(&f, 0, newSVnv(sum));
    xsub_return(1, f.ax);
}

/// `max_uv()`: the largest unsigned integer, through `newSVuv`.
unsafe extern "C-unwind" fn xs_max_uv(_cv: *mut CV) {
    let f = xsub_frame();
    set_return(&f, 0, newSVuv(UV::MAX));
    xsub_return(1, f.ax);
}

/// `to_i64($v)` / `from_i64($v)`: the `i64` conversion helpers.
unsafe extern "C-unwind" fn xs_round_trip_i64(_cv: *mut CV) {
    let f = xsub_frame();
    need_args(&f, 1, "round_trip_i64");
    set_return(&f, 0, i64_to_sv(sv_to_i64(arg(&f, 0))));
    xsub_return(1, f.ax);
}

/// `round_trip_f64($v)`: the `f64` conversion helpers.
unsafe extern "C-unwind" fn xs_round_trip_f64(_cv: *mut CV) {
    let f = xsub_frame();
    need_args(&f, 1, "round_trip_f64");
    set_return(&f, 0, f64_to_sv(sv_to_f64(arg(&f, 0))));
    xsub_return(1, f.ax);
}

// -- strings ----------------------------------------------------------------

/// `echo($s)`: the string back through `sv_to_string` / `newSVpvn` (byte exact,
/// embedded NULs included); undef when the bytes are not UTF-8 (the helper's
/// contract), with a replacement marker string.
unsafe extern "C-unwind" fn xs_echo(_cv: *mut CV) {
    let f = xsub_frame();
    need_args(&f, 1, "echo");
    let sv = match sv_to_string(arg(&f, 0)) {
        Some(s) => newSVpvn(s.as_ptr() as *const c_char, s.len()),
        None => newSVpvn(c"<not utf-8>".as_ptr(), 11),
    };
    set_return(&f, 0, sv);
    xsub_return(1, f.ax);
}

/// `cstring($s)`: the string through `newSVpv` with an explicit length (a length of
/// zero would mean `strlen`, so a non-empty input is required).
unsafe extern "C-unwind" fn xs_cstring(_cv: *mut CV) {
    let f = xsub_frame();
    need_args(&f, 1, "cstring");
    let mut len: usize = 0;
    let p = sv_2pv_flags(arg(&f, 0), &mut len, 0);
    if p.is_null() || len == 0 {
        die("cstring: need a non-empty string");
    }
    set_return(&f, 0, newSVpv(p, len));
    xsub_return(1, f.ax);
}

/// `flags($v)`: which `Sv*OK` / `SvTRUE` / `SvROK` predicates hold, as a string of
/// 0 and 1 in the order IOK, NOK, POK, TRUE, ROK.
unsafe extern "C-unwind" fn xs_flags(_cv: *mut CV) {
    let f = xsub_frame();
    need_args(&f, 1, "flags");
    let sv = arg(&f, 0);
    let bits = [SvIOK(sv), SvNOK(sv), SvPOK(sv), SvTRUE(sv), SvROK(sv) != 0];
    let mask: String = bits.iter().map(|&b| if b { '1' } else { '0' }).collect();
    set_return(&f, 0, newSVpvn(mask.as_ptr() as *const c_char, mask.len()));
    xsub_return(1, f.ax);
}

/// `count_args(...)`: the number of arguments, from `XsubFrame::items`.
unsafe extern "C-unwind" fn xs_count_args(_cv: *mut CV) {
    let f = xsub_frame();
    set_return(&f, 0, newSViv(f.items as IV));
    xsub_return(1, f.ax);
}

// -- arrays and hashes ------------------------------------------------------

/// `sum_array(\@numbers)`: the sum, through `SvROK`, `SvRV` and `av_to_f64_vec`.
unsafe extern "C-unwind" fn xs_sum_array(_cv: *mut CV) {
    let f = xsub_frame();
    need_args(&f, 1, "sum_array");
    let r = arg(&f, 0);
    if SvROK(r) == 0 {
        die("sum_array: expected an array reference");
    }
    let total = match av_to_f64_vec(SvRV(r) as *mut AV) {
        Some(values) => 0.0 + values.iter().sum::<f64>(),
        None => die("sum_array: array has a hole"),
    };
    set_return(&f, 0, newSVnv(total));
    xsub_return(1, f.ax);
}

/// `range(n)`: `[1, 2, ..., n]` as floats, through `f64_vec_to_av` / `newRV_noinc`.
unsafe extern "C-unwind" fn xs_range(_cv: *mut CV) {
    let f = xsub_frame();
    need_args(&f, 1, "range");
    let n = sv_2iv(arg(&f, 0)).max(0);
    let values: Vec<f64> = (1..=n).map(|i| i as f64).collect();
    let av = f64_vec_to_av(&values);
    set_return(&f, 0, newRV_noinc(av as *mut SV));
    xsub_return(1, f.ax);
}

/// `array_ops()`: pushes 10, 20, 30, pops one, and reports `[length_after_pushes,
/// popped, length_after_pop, element_0]` (`newAV`, `av_push`, `av_len`, `av_pop`,
/// `av_fetch`).
unsafe extern "C-unwind" fn xs_array_ops(_cv: *mut CV) {
    let f = xsub_frame();
    let av = newAV();
    for v in [10, 20, 30] {
        av_push(av, newSViv(v));
    }
    let len_after_pushes = av_len(av) + 1;
    let popped = av_pop(av);
    let popped_value = sv_2iv(popped);
    SvREFCNT_dec(popped);
    let len_after_pop = av_len(av) + 1;
    let slot = av_fetch(av, 0, 0);
    let first = if slot.is_null() { -1 } else { sv_2iv(*slot) };

    let report = newAV();
    for v in [
        len_after_pushes as IV,
        popped_value,
        len_after_pop as IV,
        first,
    ] {
        av_push(report, newSViv(v));
    }
    SvREFCNT_dec(av as *mut SV);
    set_return(&f, 0, newRV_noinc(report as *mut SV));
    xsub_return(1, f.ax);
}

/// `make_hash()`: `{ answer => 42, name => "perl-bridge", list => [1, 2] }`
/// (`new_hv`, `hv_store`, `newRV_inc` of an array also kept elsewhere).
unsafe extern "C-unwind" fn xs_make_hash(_cv: *mut CV) {
    let f = xsub_frame();
    let hv = new_hv();
    hv_store(hv, "answer", newSViv(42));
    hv_store(hv, "name", newSVpvn(c"perl-bridge".as_ptr(), 11));
    let list = f64_vec_to_av(&[1.0, 2.0]);
    hv_store(hv, "list", new_rv_inc(list as *mut SV));
    SvREFCNT_dec(list as *mut SV); // the hash's reference keeps it alive
    set_return(&f, 0, newRV_noinc(hv as *mut SV));
    xsub_return(1, f.ax);
}

/// `refcount_dance($v)`: bumps and drops a reference to `$v` and returns it.
unsafe extern "C-unwind" fn xs_refcount_dance(_cv: *mut CV) {
    let f = xsub_frame();
    need_args(&f, 1, "refcount_dance");
    let sv = arg(&f, 0);
    SvREFCNT_inc(sv);
    SvREFCNT_dec(sv);
    set_return(&f, 0, sv);
    xsub_return(1, f.ax);
}

// -- calling back into Perl -------------------------------------------------

/// `call($coderef, @args)`: calls the code reference in scalar context.  A value is
/// returned as it is, `CallResult::Empty` as the string "empty" (the call is made
/// with `G_SCALAR`, so Perl always supplies one value and this arm is not reached
/// by the test), and a `die` inside the callback as "died: <message>".
///
/// Perl may reallocate its argument stack during the call, which would leave
/// `frame.base` stale; the callbacks the test passes are tiny, so it never does.
unsafe extern "C-unwind" fn xs_call(_cv: *mut CV) {
    let f = xsub_frame();
    if f.items < 1 {
        die("call: expected a code reference");
    }
    let args: Vec<*mut SV> = (1..f.items).map(|i| arg(&f, i)).collect();
    let sv = match call_coderef(arg(&f, 0), &args) {
        CallResult::Ok(sv) => sv,
        CallResult::Empty => newSVpvn(c"empty".as_ptr(), 5),
        CallResult::Died(message) => {
            let text = format!("died: {message}");
            newSVpvn(text.as_ptr() as *const c_char, text.len())
        }
    };
    set_return(&f, 0, sv);
    xsub_return(1, f.ax);
}

/// `context_round_trip()`: 1 when the interpreter context captured with
/// `get_context` can be bound again with `set_context` (a non-threaded Perl has no
/// context: both are no-ops and the result is still 1).
unsafe extern "C-unwind" fn xs_context_round_trip(_cv: *mut CV) {
    let f = xsub_frame();
    let ctx: *mut c_void = get_context();
    set_context(ctx);
    let again = get_context();
    set_return(&f, 0, newSViv((ctx == again) as IV));
    xsub_return(1, f.ax);
}

// -- errors -----------------------------------------------------------------

/// `die_with($message)`: `die` through `perl_bridge_croak_message`.
unsafe extern "C-unwind" fn xs_die_with(_cv: *mut CV) {
    let f = xsub_frame();
    need_args(&f, 1, "die_with");
    match sv_to_string(arg(&f, 0)) {
        Some(message) => die(&message),
        None => die("die_with: message is not UTF-8"),
    }
}

/// `warn_with($message)`: `warn` through `perl_bridge_warn_message`; returns 1.
unsafe extern "C-unwind" fn xs_warn_with(_cv: *mut CV) {
    let f = xsub_frame();
    need_args(&f, 1, "warn_with");
    if let Some(message) = sv_to_string(arg(&f, 0)) {
        warn(&message);
    }
    set_return(&f, 0, newSViv(1));
    xsub_return(1, f.ax);
}

// -- boot -------------------------------------------------------------------

/// The entry point DynaLoader looks up for `PerlBridgeSmoke->bootstrap`: do the
/// handshake, register every XSUB, finish.
///
/// # Safety
///
/// Called by Perl only, once, with the CV of the `bootstrap` call.
#[no_mangle]
pub unsafe extern "C" fn boot_PerlBridgeSmoke(cv: *mut CV) {
    let file = c"PerlBridgeSmoke.xs".as_ptr();
    let ax = xs_bootstrap(cv, file);

    macro_rules! register {
        ($name:literal, $func:ident) => {
            newXS_unwind(
                concat!("PerlBridgeSmoke::", $name, "\0").as_ptr() as *const c_char,
                $func,
                file,
            );
        };
    }

    register!("add_iv", xs_add_iv);
    register!("add_nv", xs_add_nv);
    register!("max_uv", xs_max_uv);
    register!("round_trip_i64", xs_round_trip_i64);
    register!("round_trip_f64", xs_round_trip_f64);
    register!("echo", xs_echo);
    register!("cstring", xs_cstring);
    register!("flags", xs_flags);
    register!("count_args", xs_count_args);
    register!("sum_array", xs_sum_array);
    register!("range", xs_range);
    register!("array_ops", xs_array_ops);
    register!("make_hash", xs_make_hash);
    register!("refcount_dance", xs_refcount_dance);
    register!("call", xs_call);
    register!("context_round_trip", xs_context_round_trip);
    register!("die_with", xs_die_with);
    register!("warn_with", xs_warn_with);

    xs_boot_finish(ax);
}
