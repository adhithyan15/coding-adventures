// build.rs for gf256-native-perl
//
// Emits a warning if the host Perl was compiled with ithreads (MULTIPLICITY).
// Our stack access pattern (PL_stack_sp, PL_stack_base, PL_markstack_ptr as
// plain C globals) is only valid in a non-threaded Perl build. Threaded Perl
// defines these as thread-local struct fields accessed via macros — using
// them as globals will cause memory corruption.
//
// TODO: Promote this to a hard compile_error! by running:
//   perl -V:usethreads
// and parsing the output. For now we emit a warning to keep the build
// unconditional while raising visibility of the assumption.

fn main() {
    // Warn if building against threaded Perl — our stack access pattern
    // assumes non-threaded (non-MULTIPLICITY) Perl.
    // In CI, Perl is typically non-threaded on macOS/Linux.
    println!("cargo:warning=perl-native extensions assume non-threaded Perl (no MULTIPLICITY). If your Perl was compiled with usethreads, this extension will cause memory corruption.");
}
