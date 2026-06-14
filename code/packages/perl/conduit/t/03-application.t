use strict;
use warnings;
use Test::More;
use Config ();
use CodingAdventures::Conduit;

# Application construction, the chainable DSL, settings, and the internal
# _wrap / HaltError plumbing — all without binding a socket.

my $app = CodingAdventures::Conduit->new;
isa_ok($app, 'CodingAdventures::Conduit', 'new returns an Application');

# Route registration returns $self so calls chain.
is($app->get('/', sub { }), $app, 'get is chainable');
is($app->post('/x', sub { }), $app, 'post is chainable');
is($app->put('/x', sub { }), $app, 'put is chainable');
is($app->delete('/x', sub { }), $app, 'delete is chainable');
is($app->patch('/x', sub { }), $app, 'patch is chainable');
is($app->before(sub { }), $app, 'before is chainable');
is($app->after(sub { }), $app, 'after is chainable');
is($app->not_found(sub { }), $app, 'not_found is chainable');
is($app->on_error(sub { }), $app, 'on_error is chainable');

# Settings round-trip through the native store.
$app->set('views', 'tmpl');
is($app->get_setting('views'), 'tmpl', 'setting round-trips');
is($app->get_setting('missing'), undef, 'missing setting is undef');
$app->set('count', 5);
is($app->get_setting('count'), '5', 'numeric setting stringified');

# _wrap turns a handler into the env→native-response sub Rust calls.
my $wrapped = CodingAdventures::Conduit::_wrap(sub {
    my $req = shift;
    CodingAdventures::Conduit::html('hi ' . $req->path);
});
my $out = $wrapped->({ PATH_INFO => '/p' });
is(ref $out, 'ARRAY', '_wrap yields an arrayref');
is($out->[0], 200, 'wrapped status');
is($out->[1], 'hi /p', 'wrapped body (env→Request→handler)');
like($out->[2], qr/content-type=/, 'wrapped headers encoded');

# A handler returning undef passes through as undef (no match / fall-through).
my $none = CodingAdventures::Conduit::_wrap(sub { undef });
is($none->({}), undef, 'undef handler result stays undef');

# A thrown HaltError is caught in _wrap and converted to a response.
my $halter = CodingAdventures::Conduit::_wrap(sub {
    die CodingAdventures::Conduit::HaltError->new(503, 'maintenance');
});
my $h = $halter->({});
is($h->[0], 503, 'HaltError status surfaces');
is($h->[1], 'maintenance', 'HaltError body surfaces');

# A genuine die propagates (Rust routes it to on_error).
my $boom = CodingAdventures::Conduit::_wrap(sub { die "kaboom\n" });
eval { $boom->({}) };
like($@, qr/kaboom/, 'genuine die propagates out of _wrap');

# serve_background is gated: on a single-interpreter (non-MULTIPLICITY/ithreads)
# Perl it must refuse, since spawning a thread that calls the interpreter would
# corrupt it. On a threaded Perl it is allowed (we don't actually serve here).
{
    my $threaded = $Config::Config{usemultiplicity} || $Config::Config{useithreads};
    my $a2 = CodingAdventures::Conduit->new;
    $a2->get('/', sub { CodingAdventures::Conduit::text('x') });
    my $srv = $a2->bind('127.0.0.1', 0);
    if ($threaded) {
        ok(1, 'threaded Perl: serve_background gating not exercised (would spawn)');
    } else {
        eval { $srv->serve_background };
        like($@, qr/MULTIPLICITY|ithreads/,
            'serve_background croaks on a single-interpreter Perl');
    }
}

done_testing;
