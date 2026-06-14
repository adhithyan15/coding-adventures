#!/usr/bin/env perl
# ============================================================================
# conduit-hello — a full Sinatra-style demo built on CodingAdventures::Conduit.
# ============================================================================
#
# Exercises every feature of the Conduit DSL on the Perl port:
#
#   GET  /                  → HTML greeting
#   GET  /hello/:name       → JSON with the route param
#   POST /echo              → echoes the request body, content-type passthrough
#   GET  /search?q=...      → reads a query param
#   GET  /redirect          → 301 to /
#   GET  /halt              → 403 via halt()
#   GET  /down              → 503 via a before filter (short-circuits)
#   GET  /error             → dies → routed to the custom error handler (500)
#   GET  /<anything-else>   → custom 404 handler
#
# Run:   perl hello.pl              # binds 127.0.0.1:3000
#        perl hello.pl 8080         # or pick a port
# Then:  curl http://127.0.0.1:3000/hello/Adhithya
#        curl -X POST --data-binary 'ping' http://127.0.0.1:3000/echo
#
# Because Perl's default build is single-interpreter, the server runs in the
# FOREGROUND (handlers dispatch on this main thread). Press Ctrl-C to stop.

use strict;
use warnings;

$| = 1;   # autoflush STDOUT so the "listening on" line appears before serve() blocks

# Find the sibling package's lib/ relative to this script.
use FindBin qw($Bin);
use lib "$Bin/../../../packages/perl/conduit/lib";

use CodingAdventures::Conduit qw(:all);

my $port = $ARGV[0] // 3000;

my $app = CodingAdventures::Conduit->new;

$app->set('app_name', 'Conduit Hello');

# --- Before filter: short-circuit /down with a 503 -------------------------
$app->before(sub {
    my $req = shift;
    return halt(503, 'Under maintenance') if $req->path eq '/down';
    return undef;   # otherwise fall through to routing
});

# --- After filter: log each request to stderr ------------------------------
$app->after(sub {
    my $req = shift;
    print STDERR "[after] @{[ $req->method ]} @{[ $req->path ]}\n";
    return undef;
});

# --- Routes ----------------------------------------------------------------
$app->get('/', sub {
    html('<h1>Hello from Conduit (Perl)!</h1><p>Try <code>/hello/Adhithya</code></p>');
});

$app->get('/hello/:name', sub {
    my $req = shift;
    json(sprintf('{"message":"Hello %s","app":"Conduit"}', $req->param('name')));
});

$app->post('/echo', sub {
    my $req = shift;
    respond(200, $req->body, { 'content-type' => ($req->content_type || 'text/plain') });
});

$app->get('/search', sub {
    my $req = shift;
    text('you searched for: ' . (defined $req->query_param('q') ? $req->query_param('q') : '(nothing)'));
});

$app->get('/redirect', sub { redirect('/', 301) });

$app->get('/halt', sub { halt(403, 'Forbidden — this route always halts') });

$app->get('/down', sub { 'unreachable — the before filter halts first' });

$app->get('/error', sub { die "boom — something went wrong\n" });

# --- Custom not_found and error handlers -----------------------------------
$app->not_found(sub {
    my $req = shift;
    text('No such route: ' . $req->path, 404);
});

$app->on_error(sub {
    my ($req) = @_;
    json('{"error":"internal server error"}', 500);
});

my $server = $app->bind('127.0.0.1', $port);
print "conduit-hello listening on http://127.0.0.1:@{[ $server->local_port ]}/  (Ctrl-C to stop)\n";
$server->serve;
