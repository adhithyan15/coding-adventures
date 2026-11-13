use strict;
use warnings;
no strict 'refs';    # the arity table calls the XSUBs by symbolic name
use Test::More;
use IO::Socket::INET;
use CodingAdventures::Conduit;

# The native XSUBs raise Perl exceptions (`die`) for bad arity and for a failed
# bind.  A Perl `die` is a `longjmp` on Unix and an SEH unwind on Windows, so
# the Rust side must raise it from an `extern "C-unwind"` XSUB with no Rust
# frame left in between (see the "Raising Perl exceptions from an XSUB" note in
# src/lib.rs).  These tests drive every one of those paths straight through the
# native layer, the way Conduit.pm's argument handling never does, and then
# prove the interpreter is still healthy afterwards.

my $N = 'CodingAdventures::Conduit::Native';

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = eval { $code->(); 1 };
    my $err = $@;
    ok(!$ok, "$name dies");
    like($err, $pattern, "$name reports its usage");
}

my $app = CodingAdventures::Conduit::Native::new_app();
ok($app, 'new_app returns a handle');

my %arity = (
    app_add_route         => [ sub { &{"${N}::app_add_route"}($app, 'GET', '/x') },
                               qr/^app_add_route\(app, method, pattern, coderef\)/ ],
    app_add_before        => [ sub { &{"${N}::app_add_before"}($app) },
                               qr/^app_add_before\(app, coderef\)/ ],
    app_add_after         => [ sub { &{"${N}::app_add_after"}($app) },
                               qr/^app_add_after\(app, coderef\)/ ],
    app_set_not_found     => [ sub { &{"${N}::app_set_not_found"}($app) },
                               qr/^app_set_not_found\(app, coderef\)/ ],
    app_set_error_handler => [ sub { &{"${N}::app_set_error_handler"}($app) },
                               qr/^app_set_error_handler\(app, coderef\)/ ],
    app_set_setting       => [ sub { &{"${N}::app_set_setting"}($app, 'k') },
                               qr/^app_set_setting\(app, key, value\)/ ],
    app_get_setting       => [ sub { &{"${N}::app_get_setting"}($app) },
                               qr/^app_get_setting\(app, key\)/ ],
    new_server            => [ sub { &{"${N}::new_server"}($app, '127.0.0.1', 0) },
                               qr/^new_server\(app, host, port, max_conn\)/ ],
);

for my $name (sort keys %arity) {
    my ($code, $pattern) = @{ $arity{$name} };
    dies_like($code, $pattern, $name);
}

# The interpreter must be intact after every unwind above: the same app handle
# still works for a normal call.
CodingAdventures::Conduit::Native::app_set_setting($app, 'still', 'alive');
is(CodingAdventures::Conduit::Native::app_get_setting($app, 'still'), 'alive',
   'the app handle is usable after the arity errors');

# A failed bind dies too.  Occupy a port, then ask the native layer for a server
# on it.  (new_server consumes the app handle, so build a fresh one.)
my $blocker = IO::Socket::INET->new(
    LocalAddr => '127.0.0.1',
    LocalPort => 0,
    Listen    => 1,
    Proto     => 'tcp',
) or plan skip_all => "cannot open a blocking listener: $!";
my $busy_port = $blocker->sockport;

my $doomed = CodingAdventures::Conduit::Native::new_app();
my $ok = eval { CodingAdventures::Conduit::Native::new_server($doomed, '127.0.0.1', $busy_port, 8); 1 };
my $err = $@;
ok(!$ok, 'new_server on an occupied port dies');
like($err, qr/^conduit: bind failed: /, 'the bind error is reported');

# Still healthy: a brand-new app and server work after the failed bind.
my $fresh = CodingAdventures::Conduit::Native::new_app();
my $srv = CodingAdventures::Conduit::Native::new_server($fresh, '127.0.0.1', 0, 8);
ok($srv, 'a new server binds after the failed bind');
ok(CodingAdventures::Conduit::Native::server_local_port($srv) > 0, 'it reports a real port');
CodingAdventures::Conduit::Native::dispose_server($srv);

close $blocker;
done_testing();
