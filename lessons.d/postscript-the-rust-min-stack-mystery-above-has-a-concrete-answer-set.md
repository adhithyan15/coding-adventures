# Postscript: the RUST_MIN_STACK mystery above has a concrete answer — `set VAR=value &&` puts a TRAILING SPACE in the value

The entry above correctly concludes "prefer `Thread::Builder::stack_size()`
over an env var you can't verify locally," but leaves *why* the env var never
propagated as an open question after two wrong theories. It is worth naming,
because 19 other `BUILD_windows` files depend on getting this exact detail
right:

`cmd.exe`'s `SET` consumes **everything** between `=` and the `&&`, including
the space immediately before it, and does not trim. So

    set RUST_MIN_STACK=33554432 && cargo test ...

assigns the string `"33554432 "` — with a trailing space. Rust then reads it
as `env::var("RUST_MIN_STACK").ok().and_then(|s| s.parse().ok())`, and
`"33554432 ".parse::<usize>()` is an `Err`, so the value is **silently
discarded** and the default stack is used. The variable *was* set; it was just
unparseable. That is why raising 8 MiB → 32 MiB changed nothing: no stack
value was ever arriving.

The repo already encodes the correct idiom in 19 places — `set PYTHONPATH=src&&
...`, `set BISECT_FILE=%CD%\bisect&& ...`, `set PYTHONIOENCODING=utf-8&& ...`
— all with **no space before `&&`**. `smart-home-tools` was the lone deviation.
The readable spelling is the broken one, which is exactly why this recurs.

Corollaries worth keeping:

1. **A malformed env var is indistinguishable from an unset one** whenever the
   reader uses `.parse().ok()` / `unwrap_or(default)`. The symptom is "my fix
   did nothing," never an error — so confirm the value *as the process
   received it* before escalating the magnitude of the fix.
2. Still-outstanding: several `code/packages/perl/*/BUILD_windows` files use the
   spaced form for `PERL5LIB` (e.g. `set PERL5LIB=..\hash-functions\lib && perl
   ...`). Those happen to be tolerable — the value is a path list whose last
   entry merely gains a trailing space, and each also passes explicit `-I`
   flags — but they are the same latent bug and should move to `&&`-adjacent
   form if they ever start failing to find a sibling lib.
