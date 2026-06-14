use strict;
use warnings;
use Test::More;
use IO::Socket::INET;
use File::Temp qw(tempfile);
use File::Spec;
use FindBin qw($Bin);

# ============================================================================
# End-to-end: launch the real Conduit server (t/server_app.pl) as its own OS
# process, drive it with a raw HTTP/1.0 client, and assert on real responses.
# An alarm() hang guard tears the server down and fails fast if anything wedges.
# ============================================================================

my $libdir = File::Spec->catdir($Bin, File::Spec->updir, 'lib');
my $app    = File::Spec->catfile($Bin, 'server_app.pl');
my (undef, $portfile) = tempfile('conduit-portXXXXXX', TMPDIR => 1, OPEN => 0, UNLINK => 1);
unlink $portfile;   # the server creates it; we poll for its appearance

# ── Launch the server process ────────────────────────────────────────────────
my $pid = fork();
defined $pid or die "fork failed: $!";
if (!$pid) {
    # Child: become the server. exec replaces the image so there is no shared
    # interpreter state with the parent — a clean, independent process.
    exec($^X, "-I$libdir", $app, $portfile)
        or die "exec server failed: $!";
}

# Parent: client + TAP. Make sure we always reap the server.
my $reaped = 0;
my $cleanup = sub {
    return if $reaped;
    $reaped = 1;
    kill('TERM', $pid);
    waitpid($pid, 0);
};

my $port;
eval {
    local $SIG{ALRM} = sub { die "timeout waiting for server\n" };
    alarm(30);

    # Wait for the server to publish its port.
    for (1 .. 300) {
        if (-s $portfile) {
            open(my $fh, '<', $portfile) or next;
            chomp(my $line = <$fh> // '');
            close $fh;
            if ($line =~ /^(\d+)$/ && $1 > 0) { $port = $1; last; }
        }
        select(undef, undef, undef, 0.05);
    }
    die "server never published a port\n" unless $port;

    # Wait until the port actually accepts connections.
    my $up = 0;
    for (1 .. 300) {
        my $s = IO::Socket::INET->new(PeerAddr => '127.0.0.1', PeerPort => $port, Proto => 'tcp', Timeout => 1);
        if ($s) { close $s; $up = 1; last; }
        select(undef, undef, undef, 0.05);
    }
    die "server port $port never came up\n" unless $up;

    # ── Exercises ────────────────────────────────────────────────────────────
    my ($st, $hd, $bd);

    ($st, $hd, $bd) = http_req($port, 'GET', '/');
    is($st, 200, 'GET / → 200');
    is($bd, '<h1>OK</h1>', 'GET / body');
    like($hd->{'content-type'} // '', qr{text/html}, 'GET / content-type');

    ($st, $hd, $bd) = http_req($port, 'GET', '/hello/world');
    is($st, 200, 'route param → 200');
    is($bd, '{"hi":"world"}', 'route param interpolated into body');

    ($st, $hd, $bd) = http_req($port, 'POST', '/echo', 'ping-pong', 'application/octet-stream');
    is($st, 200, 'POST /echo → 200');
    is($bd, 'ping-pong', 'request body echoed back');
    like($hd->{'content-type'} // '', qr{octet-stream}, 'echo passes content-type through');

    ($st, $hd, $bd) = http_req($port, 'GET', '/q?a=42');
    is($st, 200, 'query string → 200');
    is($bd, 'a=42', 'query param parsed');

    ($st, $hd, $bd) = http_req($port, 'GET', '/down');
    is($st, 503, 'before-filter halt → 503');
    is($bd, 'maintenance', 'halt body surfaces');

    ($st, $hd, $bd) = http_req($port, 'GET', '/boom');
    is($st, 500, 'dying handler → on_error 500');
    is($bd, '{"error":"server"}', 'on_error body');

    ($st, $hd, $bd) = http_req($port, 'GET', '/nope');
    is($st, 404, 'unknown route → custom not_found 404');
    is($bd, 'no route: /nope', 'not_found body includes path');

    ($st, $hd, $bd) = http_req($port, 'GET', '/redir');
    is($st, 302, 'redirect → 302');
    is($hd->{'location'} // '', '/', 'redirect Location header');

    alarm(0);
};
my $err = $@;
$cleanup->();
ok(!$err, 'E2E run completed without timeout/exception') or diag($err);

done_testing;

# ── Minimal HTTP/1.0 client ──────────────────────────────────────────────────
# HTTP/1.0 + "Connection: close" lets us read the whole response to EOF.
sub http_req {
    my ($port, $method, $path, $body, $ctype) = @_;
    my $sock = IO::Socket::INET->new(
        PeerAddr => '127.0.0.1', PeerPort => $port, Proto => 'tcp', Timeout => 5,
    ) or die "connect to $port failed: $!";
    $body //= '';
    my $req = "$method $path HTTP/1.0\r\nHost: 127.0.0.1\r\n";
    if (length $body) {
        $ctype //= 'text/plain';
        $req .= "Content-Type: $ctype\r\nContent-Length: " . length($body) . "\r\n";
    }
    $req .= "Connection: close\r\n\r\n$body";
    print $sock $req;
    my $raw = do { local $/; <$sock> };
    close $sock;
    $raw //= '';
    my ($head, $rbody) = split /\r\n\r\n/, $raw, 2;
    $rbody //= '';
    my @lines = split /\r\n/, $head;
    my $status_line = shift @lines // '';
    my ($status) = $status_line =~ m{^HTTP/\d\.\d\s+(\d+)};
    my %h;
    for my $l (@lines) {
        my ($k, $v) = split /:\s*/, $l, 2;
        $h{ lc $k } = $v if defined $v;
    }
    return ($status // 0, \%h, $rbody);
}
