use strict;
use warnings;
use Test::More;
use CodingAdventures::Conduit::Request;

# A Request is a read-only view over the flat env hashref Rust passes in.
my $env = {
    REQUEST_METHOD        => 'POST',
    PATH_INFO             => '/hello/world',
    QUERY_STRING          => 'a=1&b=two+words',
    REMOTE_ADDR           => '127.0.0.1:5050',
    'conduit.body'        => 'payload',
    'conduit.content_type' => 'text/plain',
    'conduit.error'       => 'boom',
    'conduit.route_params' => 'name=world',
    'conduit.query_params' => 'a=1&b=two+words',
    'conduit.headers'     => 'content-type=text%2Fplain&x-custom=hi',
};
my $req = CodingAdventures::Conduit::Request->_new($env);

is($req->method, 'POST', 'method');
is($req->path, '/hello/world', 'path');
is($req->query_string, 'a=1&b=two+words', 'query_string');
is($req->body, 'payload', 'body');
is($req->content_type, 'text/plain', 'content_type');
is($req->remote_addr, '127.0.0.1:5050', 'remote_addr');
is($req->error, 'boom', 'error');

is($req->param('name'), 'world', 'route param decoded');
is($req->query_param('a'), '1', 'query param a');
is($req->query_param('b'), 'two words', 'query param b (+ → space)');
is($req->header('content-type'), 'text/plain', 'header decoded (%2F → /)');
is($req->header('Content-Type'), 'text/plain', 'header lookup is case-insensitive');
is($req->header('x-custom'), 'hi', 'custom header');

# Defaults for an empty env.
my $empty = CodingAdventures::Conduit::Request->_new({});
is($empty->method, 'GET', 'default method GET');
is($empty->path, '/', 'default path /');
is($empty->body, '', 'default body empty');
is_deeply($empty->params, {}, 'empty params');
is_deeply($empty->query_params, {}, 'empty query_params');
is($empty->param('missing'), undef, 'missing param is undef');

# %XX percent-decoding round-trips arbitrary bytes.
my $r2 = CodingAdventures::Conduit::Request->_new({ 'conduit.route_params' => 'k=a%26b%3Dc' });
is($r2->param('k'), 'a&b=c', 'percent-encoded delimiters survive decode');

done_testing;
