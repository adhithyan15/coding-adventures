use strict;
use warnings;
use Config;
use File::Copy qw(copy);
use File::Path qw(make_path);
use File::Spec;
use FindBin;
use Test::More;

# tests/smoke.pl -- load the perl_bridge_smoke cdylib into a real Perl and drive
# every part of perl-bridge it uses.
#
#   perl tests/smoke.pl <directory that holds the built cdylib>
#
# BUILD builds the library with `cargo build --release --example
# perl_bridge_smoke --target-dir target`, so the directory is
# target/release/examples.  cargo names the file per platform
# (libperl_bridge_smoke.so, libperl_bridge_smoke.dylib, perl_bridge_smoke.dll) but
# DynaLoader wants auto/PerlBridgeSmoke/PerlBridgeSmoke.$Config{dlext} (.so, .bundle
# on macOS, .dll on Windows), so the script stages a copy there first, under
# target/smoke-lib next to the cdylib's directory.
#
# Test::More ships with Perl, so the script needs no CPAN module.  A failed check
# fails the script (and the BUILD); there is no skip path.

my $dir = shift @ARGV;
die "usage: perl tests/smoke.pl <cdylib directory>\n" unless defined $dir;

my $source;
for my $candidate (
    "libperl_bridge_smoke.$Config{so}",    # Linux .so, macOS .dylib
    'libperl_bridge_smoke.so',
    'perl_bridge_smoke.dll',               # Windows
) {
    my $path = File::Spec->catfile($dir, $candidate);
    if (-f $path) {
        $source = $path;
        last;
    }
}
die "no perl_bridge_smoke library in $dir\n" unless defined $source;

my $lib = File::Spec->catdir($dir, File::Spec->updir, File::Spec->updir, 'smoke-lib');
my $auto = File::Spec->catdir($lib, 'auto', 'PerlBridgeSmoke');
make_path($auto);
copy($source, File::Spec->catfile($auto, "PerlBridgeSmoke.$Config{dlext}"))
    or die "cannot stage $source into $auto: $!\n";

unshift @INC, $lib, File::Spec->catdir($FindBin::Bin, 'lib');
require PerlBridgeSmoke;

# numbers
is(PerlBridgeSmoke::add_iv(40, 2), 42, 'add_iv');
is(PerlBridgeSmoke::add_iv(-10, 3), -7, 'add_iv with a negative');
is(PerlBridgeSmoke::add_iv('7', '8'), 15, 'add_iv coerces numeric strings');
is(PerlBridgeSmoke::add_nv(1.5, 2.25), 3.75, 'add_nv');
is(PerlBridgeSmoke::max_uv(), ~0, 'newSVuv holds the largest unsigned integer');
is(PerlBridgeSmoke::round_trip_i64(-5), -5, 'i64 helpers round trip a negative');
is(PerlBridgeSmoke::round_trip_i64(9_007_199_254_740_993), 9_007_199_254_740_993,
    'i64 helpers keep 2**53 + 1 exactly (an f64 cannot)');
is(PerlBridgeSmoke::round_trip_f64(0.1), 0.1, 'f64 helpers round trip a float');

# strings
is(PerlBridgeSmoke::echo('hello'), 'hello', 'echo');
is(PerlBridgeSmoke::echo("a\0b\0c"), "a\0b\0c", 'echo keeps embedded NULs');
is(length(PerlBridgeSmoke::echo("a\0b\0c")), 5, 'echo length with NULs');
is(PerlBridgeSmoke::echo(''), '', 'echo of the empty string');
is(PerlBridgeSmoke::echo(42), '42', 'echo coerces a number to its string');
is(PerlBridgeSmoke::echo("\xff\xfe"), '<not utf-8>', 'echo reports bytes that are not UTF-8');
is(PerlBridgeSmoke::cstring('abc'), 'abc', 'newSVpv with an explicit length');

# predicates: IOK NOK POK TRUE ROK
is(PerlBridgeSmoke::flags(5), '10010', 'flags of an integer');
is(PerlBridgeSmoke::flags(1.5), '01010', 'flags of a float');
is(PerlBridgeSmoke::flags('x'), '00110', 'flags of a string');
is(PerlBridgeSmoke::flags(0), '10000', 'flags of 0: false');
is(PerlBridgeSmoke::flags(''), '00100', 'flags of the empty string: false');
is(PerlBridgeSmoke::flags([]), '00011', 'flags of a reference');
is(PerlBridgeSmoke::count_args(), 0, 'count_args with none');
is(PerlBridgeSmoke::count_args(1, 2, 3), 3, 'count_args with three');

# arrays and hashes
is(PerlBridgeSmoke::sum_array([1, 2, 3.5]), 6.5, 'sum_array');
is(PerlBridgeSmoke::sum_array([]), 0, 'sum_array of an empty array');
ok(!eval { PerlBridgeSmoke::sum_array(5); 1 }, 'sum_array rejects a non-reference');
like($@, qr/expected an array reference/, 'and says why');
is_deeply(PerlBridgeSmoke::range(4), [1, 2, 3, 4], 'range');
is_deeply(PerlBridgeSmoke::range(0), [], 'range(0)');
is_deeply(PerlBridgeSmoke::array_ops(), [3, 30, 2, 10], 'av_push, av_len, av_pop, av_fetch');
{
    my $h = PerlBridgeSmoke::make_hash();
    is(ref $h, 'HASH', 'make_hash returns a hash reference');
    is($h->{answer}, 42, 'hv_store integer');
    is($h->{name}, 'perl-bridge', 'hv_store string');
    is_deeply($h->{list}, [1, 2], 'hv_store of a shared array reference');
    is(scalar(keys %$h), 3, 'three keys');
}
is(PerlBridgeSmoke::refcount_dance(7), 7, 'SvREFCNT_inc / SvREFCNT_dec leave the value alone');

# calling back into Perl
is(PerlBridgeSmoke::call(sub { $_[0] * $_[1] }, 6, 7), 42, 'call_coderef returns the callback value');
is(PerlBridgeSmoke::call(sub { 'plain' }), 'plain', 'call_coderef with no arguments');
# The call is made in scalar context, so a callback that returns nothing still
# yields one value, undef (CallResult::Empty is for a count of zero, which G_SCALAR
# never produces).
is(PerlBridgeSmoke::call(sub { return }), undef, 'call_coderef: an empty return is undef in scalar context');
like(PerlBridgeSmoke::call(sub { die "kaboom\n" }), qr/^died: kaboom/, 'call_coderef reports a die');
is(PerlBridgeSmoke::call(sub { 1 }), 1, 'the interpreter is usable after a die in a callback');
is(PerlBridgeSmoke::context_round_trip(), 1, 'get_context / set_context');

# errors and warnings: croak unwinds into eval, warn reaches $SIG{__WARN__}
ok(!eval { PerlBridgeSmoke::die_with('from rust'); 1 }, 'die_with dies');
like($@, qr/^from rust/, 'with the message');
ok(!eval { PerlBridgeSmoke::add_iv(1); 1 }, 'a wrong argument count dies');
like($@, qr/add_iv: expected 2 arguments, got 1/, 'with the formatted message');
{
    my @warnings;
    local $SIG{__WARN__} = sub { push @warnings, @_ };
    is(PerlBridgeSmoke::warn_with('careful'), 1, 'warn_with returns');
    is(scalar(@warnings), 1, 'one warning');
    like($warnings[0], qr/^careful/, 'with the message');
}
is(PerlBridgeSmoke::add_iv(1, 1), 2, 'the interpreter is usable after errors');

done_testing();
