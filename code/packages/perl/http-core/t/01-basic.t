use strict;
use warnings;

use Test2::V0;

use lib 'lib';
use CodingAdventures::HttpCore qw(find_header parse_content_length parse_content_type);

my $version = CodingAdventures::HttpCore::HttpVersion->parse('HTTP/1.1');
is($version->{major}, 1, 'major version parsed');
is($version->{minor}, 1, 'minor version parsed');
is($version->as_string, 'HTTP/1.1', 'version renders back to text');

my $headers = [
    CodingAdventures::HttpCore::Header->new(name => 'Content-Length', value => '42'),
    CodingAdventures::HttpCore::Header->new(name => 'Content-Type', value => 'text/html; charset=utf-8'),
];

is(find_header($headers, 'content-length'), '42', 'header lookup is case insensitive');
is(parse_content_length($headers), 42, 'content length parses');

my ($media_type, $charset) = parse_content_type($headers);
is($media_type, 'text/html', 'media type parsed');
is($charset, 'utf-8', 'charset parsed');

my $request = CodingAdventures::HttpCore::RequestHead->new(
    method  => 'POST',
    target  => '/submit',
    version => CodingAdventures::HttpCore::HttpVersion->new(major => 1, minor => 1),
    headers => [CodingAdventures::HttpCore::Header->new(name => 'Content-Length', value => '5')],
);
is($request->content_length, 5, 'request delegates content length helper');

my $body_kind = CodingAdventures::HttpCore::BodyKind->content_length(7);
is($body_kind->{mode}, 'content-length', 'body kind stores mode');
is($body_kind->{length}, 7, 'body kind stores length');

done_testing;
