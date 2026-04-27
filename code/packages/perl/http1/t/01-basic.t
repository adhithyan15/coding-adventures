use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Http1;
use CodingAdventures::HttpCore;

my $request = CodingAdventures::Http1::parse_request_head("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n");
is($request->{head}->{method}, 'GET', 'parses request method');
is($request->{head}->{target}, '/', 'parses request target');
is($request->{body_kind}->{mode}, 'none', 'simple request has no body');

my $post = CodingAdventures::Http1::parse_request_head("POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello");
is($post->{body_kind}->{mode}, 'content-length', 'request body mode uses content-length');
is($post->{body_kind}->{length}, 5, 'request body length is preserved');

my $response = CodingAdventures::Http1::parse_response_head("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody");
is($response->{head}->{status}, 200, 'parses response status');
is($response->{head}->{reason}, 'OK', 'parses response reason');
is($response->{body_kind}->{mode}, 'content-length', 'response body mode uses content-length');

my $streamed = CodingAdventures::Http1::parse_response_head("HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n");
is($streamed->{body_kind}->{mode}, 'until-eof', 'missing response length reads until eof');

my $bodyless = CodingAdventures::Http1::parse_response_head("HTTP/1.1 204 No Content\r\nContent-Length: 12\r\n\r\n");
is($bodyless->{body_kind}->{mode}, 'none', 'bodyless status overrides content length');

my $duplicate = CodingAdventures::Http1::parse_response_head("\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload");
is([ map { $_->{value} } @{$duplicate->{head}->{headers}} ], ['a=1', 'b=2'], 'duplicate headers are preserved');

like(
    dies { CodingAdventures::Http1::parse_request_head("GET / HTTP/1.1\r\nHost example.com\r\n\r\n") },
    qr/invalid HTTP\/1 header/,
    'rejects invalid headers',
);

like(
    dies { CodingAdventures::Http1::parse_response_head("HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n") },
    qr/invalid Content-Length/,
    'rejects invalid content length',
);

done_testing;
