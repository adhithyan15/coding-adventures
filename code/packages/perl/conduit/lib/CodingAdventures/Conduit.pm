package CodingAdventures::Conduit;

# ============================================================================
# CodingAdventures::Conduit — a Sinatra-style web framework for Perl
# ============================================================================
#
# Wraps the Rust web-core engine (via the WEB08 conduit facade) through an XS
# native library. Handlers are Perl subs; routing, lifecycle hooks, and HTTP
# I/O run in Rust. See code/specs/WEB11-conduit-perl.md.
#
#   use CodingAdventures::Conduit qw(:all);
#
#   my $app = CodingAdventures::Conduit->new;
#   $app->before(sub { my $req = shift; $req->path eq '/down' ? halt(503, 'down') : undef });
#   $app->get('/', sub { html('<h1>Hello from Conduit!</h1>') });
#   $app->get('/hello/:name', sub { my $req = shift; json(qq({"hi":"@{[$req->param('name')]}"})) });
#   $app->post('/echo', sub { my $req = shift; respond(200, $req->body, { 'content-type' => $req->content_type }) });
#   $app->not_found(sub { my $req = shift; html('Not Found: ' . $req->path, 404) });
#   $app->on_error(sub { json('{"error":"oops"}', 500) });
#   my $server = $app->bind('127.0.0.1', 3000);
#   $server->serve;   # blocks until stopped

use strict;
use warnings;
use DynaLoader;

our $VERSION = '0.01';
our @ISA = ('DynaLoader');

sub dl_load_flags { 0x01 }

__PACKAGE__->bootstrap($VERSION);

use CodingAdventures::Conduit::Request;

use Exporter 'import';
our @EXPORT_OK = qw(html json text respond halt redirect);
our %EXPORT_TAGS = (all => \@EXPORT_OK);

# ── Response helpers — return [status, headers_hashref, body] ────────────────

sub html    { my ($b, $s) = @_; [ $s // 200, { 'content-type' => 'text/html; charset=utf-8' },  defined $b ? $b : '' ] }
sub json    { my ($b, $s) = @_; [ $s // 200, { 'content-type' => 'application/json' },          defined $b ? $b : '' ] }
sub text    { my ($b, $s) = @_; [ $s // 200, { 'content-type' => 'text/plain; charset=utf-8' }, defined $b ? $b : '' ] }
sub respond { my ($s, $b, $h) = @_; [ $s, $h // {}, defined $b ? $b : '' ] }
sub halt    { my ($s, $b) = @_; [ $s, { 'content-type' => 'text/plain; charset=utf-8' }, defined $b ? $b : '' ] }

sub redirect {
    my ($location, $status) = @_;
    die "redirect location must not contain CR or LF\n"
        if defined $location && $location =~ /[\r\n]/;
    [ $status // 302, { location => $location }, '' ];
}

# ── Application ──────────────────────────────────────────────────────────────

sub new {
    my ($class) = @_;
    my $app = CodingAdventures::Conduit::Native::new_app();
    bless { _app => $app, _consumed => 0 }, $class;
}

for my $method (qw(get post put delete patch)) {
    no strict 'refs';
    *{$method} = sub {
        my ($self, $pattern, $handler) = @_;
        CodingAdventures::Conduit::Native::app_add_route(
            $self->{_app}, uc $method, $pattern, _wrap($handler));
        return $self;
    };
}

sub before {
    my ($self, $h) = @_;
    CodingAdventures::Conduit::Native::app_add_before($self->{_app}, _wrap($h));
    return $self;
}

sub after {
    my ($self, $h) = @_;
    CodingAdventures::Conduit::Native::app_add_after($self->{_app}, _wrap($h));
    return $self;
}

sub not_found {
    my ($self, $h) = @_;
    CodingAdventures::Conduit::Native::app_set_not_found($self->{_app}, _wrap($h));
    return $self;
}

sub on_error {
    my ($self, $h) = @_;
    CodingAdventures::Conduit::Native::app_set_error_handler($self->{_app}, _wrap($h));
    return $self;
}

sub set {
    my ($self, $key, $value) = @_;
    CodingAdventures::Conduit::Native::app_set_setting($self->{_app}, "$key", "$value");
    return $self;
}

sub get_setting {
    my ($self, $key) = @_;
    return CodingAdventures::Conduit::Native::app_get_setting($self->{_app}, "$key");
}

sub bind {
    my ($self, $host, $port, $max) = @_;
    $host //= '127.0.0.1';
    $port //= 3000;
    $max  //= 128;
    my $srv = CodingAdventures::Conduit::Native::new_server($self->{_app}, $host, $port, $max);
    $self->{_consumed} = 1;
    return CodingAdventures::Conduit::Server->_new($srv);
}

sub DESTROY {
    my ($self) = @_;
    if (!$self->{_consumed} && $self->{_app}) {
        CodingAdventures::Conduit::Native::dispose_app($self->{_app});
        $self->{_app} = 0;
    }
}

# Wrap a user handler into the sub the Rust side calls with the env hashref.
# Builds a Request, runs the handler under eval, converts a thrown HaltError
# into a response, and re-dies genuine errors (which Rust routes to on_error).
sub _wrap {
    my ($handler) = @_;
    return sub {
        my ($env) = @_;
        my $req = CodingAdventures::Conduit::Request->_new($env);
        my $resp = eval { $handler->($req) };
        if (my $e = $@) {
            if (ref $e && eval { $e->isa('CodingAdventures::Conduit::HaltError') }) {
                return _native_response($e->to_response);
            }
            die $e;
        }
        return undef unless defined $resp;
        return _native_response($resp);
    };
}

# Convert [status, headers_hashref, body] → [status, body, headers_encoded].
sub _native_response {
    my ($r) = @_;
    return undef unless defined $r && ref $r eq 'ARRAY';
    my ($status, $headers, $body) = @$r;
    return [ $status, defined $body ? "$body" : '', _encode_headers($headers // {}) ];
}

sub _encode_headers {
    my ($h) = @_;
    my @parts;
    for my $k (sort keys %$h) {
        my $v = $h->{$k};
        next if $k =~ /[\r\n]/ || (defined $v && $v =~ /[\r\n]/);
        push @parts, _pct(lc $k) . '=' . _pct(defined $v ? $v : '');
    }
    return join('&', @parts);
}

sub _pct {
    my ($s) = @_;
    $s =~ s/([^A-Za-z0-9\-_.~])/sprintf('%%%02X', ord($1))/ge;
    return $s;
}

# ── HaltError — for non-local Sinatra-style halts (die with this) ────────────

package CodingAdventures::Conduit::HaltError;

sub new {
    my ($class, $status, $body, $headers) = @_;
    bless { status => $status, body => $body // '', headers => $headers // {} }, $class;
}
sub to_response { my ($s) = @_; [ $s->{status}, $s->{headers}, $s->{body} ] }

# ── Server ───────────────────────────────────────────────────────────────────

package CodingAdventures::Conduit::Server;

use Config ();

sub _new { my ($class, $srv) = @_; bless { _srv => $srv }, $class }
sub serve            { CodingAdventures::Conduit::Native::server_serve($_[0]{_srv}) }

# serve_background spawns an OS thread in Rust that calls back into the Perl
# interpreter. That is only sound on a MULTIPLICITY/ithreads-capable build —
# a single-interpreter Perl is bound to the thread that initialized it, and
# dispatching handlers from another thread corrupts/crashes it. So we gate the
# native call on the build and croak with guidance otherwise. For concurrent
# testing on a non-threaded Perl, run serve() in the foreground and drive it
# from a separate client process (see t/04-server.t).
sub serve_background {
    my ($self) = @_;
    unless ($Config::Config{usemultiplicity} || $Config::Config{useithreads}) {
        require Carp;
        Carp::croak(
            "serve_background requires a Perl built with MULTIPLICITY or ithreads; "
          . "this interpreter has neither. Use serve() in the foreground (optionally "
          . "from a forked/separate process) instead."
        );
    }
    CodingAdventures::Conduit::Native::server_serve_background($self->{_srv});
    return $self;
}
sub stop             { CodingAdventures::Conduit::Native::server_stop($_[0]{_srv}) }
sub local_port       { CodingAdventures::Conduit::Native::server_local_port($_[0]{_srv}) }
sub running          { CodingAdventures::Conduit::Native::server_running($_[0]{_srv}) ? 1 : 0 }

sub DESTROY {
    my ($self) = @_;
    if ($self->{_srv}) {
        CodingAdventures::Conduit::Native::dispose_server($self->{_srv});
        $self->{_srv} = 0;
    }
}

1;
