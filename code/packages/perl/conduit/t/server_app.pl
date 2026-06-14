#!/usr/bin/env perl
# ============================================================================
# t/server_app.pl — the Conduit application under test, as its own process.
# ============================================================================
#
# Run as:  perl -Ilib t/server_app.pl <portfile>
#
# We bind to 127.0.0.1:0 (OS-assigned port), write the chosen port to
# <portfile> so the test harness knows where to connect, then serve in the
# FOREGROUND. Foreground serve runs the inline reactor on this process's main
# (and only) interpreter thread, so handlers dispatch on the original thread —
# the dispatch model a non-threaded Perl requires. Running the server as a
# separate OS process (rather than a thread) keeps TAP output clean and sidesteps
# every single-interpreter threading concern.

use strict;
use warnings;
use lib 'lib';
use CodingAdventures::Conduit qw(:all);

my $portfile = $ARGV[0] or die "usage: server_app.pl <portfile>\n";

my $app = CodingAdventures::Conduit->new;

# A before-filter that can short-circuit with halt() (Sinatra-style).
$app->before(sub {
    my $req = shift;
    return halt(503, 'maintenance') if $req->path eq '/down';
    return undef;
});

$app->get('/', sub { html('<h1>OK</h1>') });

# Route params.
$app->get('/hello/:name', sub {
    my $req = shift;
    json(sprintf('{"hi":"%s"}', $req->param('name')));
});

# Request body echo with content-type passthrough.
$app->post('/echo', sub {
    my $req = shift;
    respond(200, $req->body, { 'content-type' => ($req->content_type || 'text/plain') });
});

# Query string.
$app->get('/q', sub {
    my $req = shift;
    text('a=' . (defined $req->query_param('a') ? $req->query_param('a') : ''));
});

# A handler that dies → routed to on_error.
$app->get('/boom', sub { die "explode\n" });

# Redirect.
$app->get('/redir', sub { redirect('/', 302) });

$app->not_found(sub { my $req = shift; text('no route: ' . $req->path, 404) });
$app->on_error(sub { json('{"error":"server"}', 500) });

my $server = $app->bind('127.0.0.1', 0);

open(my $pf, '>', $portfile) or die "cannot write portfile $portfile: $!";
print $pf $server->local_port, "\n";
close $pf;

$server->serve;   # blocks until SIGTERM
