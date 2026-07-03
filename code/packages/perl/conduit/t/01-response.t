use strict;
use warnings;
use Test::More;
use CodingAdventures::Conduit qw(:all);

# Response helpers return [status, headers_hashref, body].

my $h = html('<h1>Hi</h1>');
is($h->[0], 200, 'html default status 200');
is($h->[1]{'content-type'}, 'text/html; charset=utf-8', 'html content-type');
is($h->[2], '<h1>Hi</h1>', 'html body');
is(html('x', 201)->[0], 201, 'html explicit status');

my $j = json('{"ok":1}');
is($j->[1]{'content-type'}, 'application/json', 'json content-type');
is($j->[2], '{"ok":1}', 'json body');
is(json('{}', 500)->[0], 500, 'json explicit status');

my $t = text('pong');
is($t->[1]{'content-type'}, 'text/plain; charset=utf-8', 'text content-type');

my $r = respond(204, '', { 'x-y' => 'z' });
is($r->[0], 204, 'respond status');
is($r->[1]{'x-y'}, 'z', 'respond headers');

my $hl = halt(403, 'no');
is($hl->[0], 403, 'halt status');
is($hl->[2], 'no', 'halt body');

my $rd = redirect('/new');
is($rd->[0], 302, 'redirect default 302');
is($rd->[1]{location}, '/new', 'redirect location');
is(redirect('/old', 301)->[0], 301, 'redirect explicit status');

eval { redirect("/x\r\nSet-Cookie: evil=1") };
like($@, qr/CR or LF/, 'redirect rejects CRLF (response-splitting guard)');

# header encoding lowercases names, drops CRLF-bearing headers
is(CodingAdventures::Conduit::_encode_headers({ 'X-A' => 'b' }), 'x-a=b', 'header name lowercased');
is(CodingAdventures::Conduit::_encode_headers({ 'x' => "a\r\nb" }), '', 'CRLF header dropped');
like(CodingAdventures::Conduit::_encode_headers({ 'content-type' => 'text/html' }), qr/%2F/, 'slash percent-encoded');

done_testing;
