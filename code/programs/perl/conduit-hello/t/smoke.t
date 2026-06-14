use strict;
use warnings;
use Test::More;
use IO::Socket::INET;
use File::Spec;
use FindBin qw($Bin);

# Smoke test: launch the real conduit-hello demo on an OS-assigned port, hit a
# couple of routes, and confirm it actually serves. An alarm() guard fails fast
# if the server wedges, and we always reap the child.

my $hello = File::Spec->catfile($Bin, File::Spec->updir, 'hello.pl');

# Launch demo on port 0; it prints "...http://127.0.0.1:<port>/..." to stdout.
my $pid = open(my $out, '-|', $^X, $hello, '0')
    or die "cannot launch demo: $!";

my $reaped = 0;
my $cleanup = sub { return if $reaped; $reaped = 1; kill('TERM', $pid); waitpid($pid, 0); };

my $port;
eval {
    local $SIG{ALRM} = sub { die "timeout\n" };
    alarm(30);

    while (my $line = <$out>) {
        if ($line =~ m{127\.0\.0\.1:(\d+)/}) { $port = $1; last; }
    }
    die "demo never reported a port\n" unless $port;

    my ($st, $bd) = get($port, '/');
    is($st, 200, 'GET / → 200');
    like($bd, qr/Hello from Conduit/, 'greeting body');

    ($st, $bd) = get($port, '/hello/Ada');
    is($st, 200, 'GET /hello/:name → 200');
    like($bd, qr/Hello Ada/, 'route param interpolated');

    ($st, $bd) = get($port, '/nope');
    is($st, 404, 'unknown route → custom 404');

    alarm(0);
};
my $err = $@;
$cleanup->();
ok(!$err, 'smoke run completed without timeout') or diag($err);

done_testing;

sub get {
    my ($port, $path) = @_;
    my $s = IO::Socket::INET->new(PeerAddr => '127.0.0.1', PeerPort => $port, Proto => 'tcp', Timeout => 5)
        or die "connect failed: $!";
    print $s "GET $path HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    my $raw = do { local $/; <$s> };
    close $s;
    $raw //= '';
    my ($head, $body) = split /\r\n\r\n/, $raw, 2;
    my ($status) = ($head // '') =~ m{^HTTP/\d\.\d\s+(\d+)};
    return ($status // 0, $body // '');
}
