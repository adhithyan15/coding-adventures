#!/usr/bin/env perl

# 06_server.t -- Server integration tests

use strict;
use warnings;
use utf8;
use Test::More;

use JSON::PP qw(encode_json decode_json);

use CodingAdventures::Ls00::Server;
use CodingAdventures::Ls00::Capabilities qw(build_capabilities);

# ── TestBridge ───────────────────────────────────────────────────────────────

{
    package TestBridge;

    sub new {
        my ($class, %args) = @_;
        return bless {
            hover_result => $args{hover_result},
        }, $class;
    }

    sub tokenize {
        my ($self, $source) = @_;
        my @tokens;
        my $col = 1;
        for my $word (split /\s+/, $source) {
            push @tokens, {
                type   => 'WORD',
                value  => $word,
                line   => 1,
                column => $col,
            };
            $col += length($word) + 1;
        }
        return (\@tokens, undef);
    }

    sub parse {
        my ($self, $source) = @_;
        my @diags;
        if ($source =~ /ERROR/) {
            push @diags, {
                range => {
                    start => { line => 0, character => 0 },
                    end   => { line => 0, character => 5 },
                },
                severity => 1,
                message  => 'syntax error: unexpected ERROR token',
            };
        }
        return ($source, \@diags, undef);
    }

    sub hover {
        my ($self, $ast, $pos) = @_;
        return ($self->{hover_result}, undef);
    }

    sub document_symbols {
        my ($self, $ast) = @_;
        return ([
            {
                name => 'main',
                kind => 12,
                range => {
                    start => { line => 0, character => 0 },
                    end   => { line => 10, character => 1 },
                },
                selection_range => {
                    start => { line => 0, character => 9 },
                    end   => { line => 0, character => 13 },
                },
                children => [
                    {
                        name => 'x',
                        kind => 13,
                        range => {
                            start => { line => 1, character => 4 },
                            end   => { line => 1, character => 12 },
                        },
                        selection_range => {
                            start => { line => 1, character => 8 },
                            end   => { line => 1, character => 9 },
                        },
                        children => [],
                    },
                ],
            },
        ], undef);
    }
}

# ── Helpers ──────────────────────────────────────────────────────────────────

sub make_message {
    my ($obj) = @_;
    my $json = encode_json($obj);
    my $len  = length($json);
    return "Content-Length: $len\r\n\r\n$json";
}

sub read_message_from_string {
    my ($str_ref) = @_;
    if ($$str_ref =~ s/^Content-Length:\s*(\d+)\r\n\r\n//s) {
        my $len = $1;
        my $payload = substr($$str_ref, 0, $len, '');
        return decode_json($payload);
    }
    return undef;
}

# ── Capabilities Tests ──────────────────────────────────────────────────────

subtest "build capabilities for test bridge" => sub {
    my $bridge = TestBridge->new(
        hover_result => { contents => '**main** function' },
    );
    my $caps = build_capabilities($bridge);

    is($caps->{textDocumentSync}, 2, "textDocumentSync = 2");
    ok($caps->{hoverProvider}, "hoverProvider advertised");
    ok($caps->{documentSymbolProvider}, "documentSymbolProvider advertised");
    ok(!exists $caps->{definitionProvider}, "no definitionProvider");
    ok(!exists $caps->{referencesProvider}, "no referencesProvider");
    ok(!exists $caps->{completionProvider}, "no completionProvider");
    ok(!exists $caps->{renameProvider}, "no renameProvider");
};

# ── Initialize returns capabilities ─────────────────────────────────────────

subtest "initialize returns capabilities" => sub {
    my $bridge = TestBridge->new(
        hover_result => { contents => '**main** function' },
    );

    my $input_str = make_message({
        jsonrpc => '2.0',
        id      => 1,
        method  => 'initialize',
        params  => {
            processId    => 12345,
            capabilities => {},
        },
    });

    open my $in_fh,  '<', \$input_str  or die "Cannot open input: $!";
    my $output_str = '';
    open my $out_fh, '>', \$output_str or die "Cannot open output: $!";
    binmode($in_fh, ':raw');
    binmode($out_fh, ':raw');

    my $server = CodingAdventures::Ls00::Server->new($bridge, $in_fh, $out_fh);
    $server->serve();

    my $resp = read_message_from_string(\$output_str);
    ok($resp, "got a response");
    is($resp->{id}, 1, "response id matches");

    my $result = $resp->{result};
    ok($result, "result present");
    ok($result->{capabilities}, "capabilities present");
    is($result->{capabilities}{textDocumentSync}, 2, "textDocumentSync = 2");
    ok($result->{capabilities}{hoverProvider}, "hoverProvider in capabilities");
    ok($result->{capabilities}{documentSymbolProvider}, "documentSymbolProvider in capabilities");

    ok($result->{serverInfo}, "serverInfo present");
    is($result->{serverInfo}{name}, 'ls00-generic-lsp-server', "server name correct");
};

# ── didOpen publishes diagnostics ────────────────────────────────────────────

subtest "didOpen publishes diagnostics" => sub {
    my $bridge = TestBridge->new();

    my $input_str = '';
    $input_str .= make_message({
        jsonrpc => '2.0',
        id      => 1,
        method  => 'initialize',
        params  => { processId => 1, capabilities => {} },
    });
    $input_str .= make_message({
        jsonrpc => '2.0',
        method  => 'initialized',
        params  => {},
    });
    $input_str .= make_message({
        jsonrpc => '2.0',
        method  => 'textDocument/didOpen',
        params  => {
            textDocument => {
                uri        => 'file:///test.txt',
                languageId => 'test',
                version    => 1,
                text       => 'some ERROR here',
            },
        },
    });

    open my $in_fh,  '<', \$input_str  or die "Cannot open input: $!";
    my $output_str = '';
    open my $out_fh, '>', \$output_str or die "Cannot open output: $!";
    binmode($in_fh, ':raw');
    binmode($out_fh, ':raw');

    my $server = CodingAdventures::Ls00::Server->new($bridge, $in_fh, $out_fh);
    $server->serve();

    my $resp1 = read_message_from_string(\$output_str);
    ok($resp1, "got initialize response");
    is($resp1->{id}, 1, "initialize response id = 1");

    my $resp2 = read_message_from_string(\$output_str);
    ok($resp2, "got diagnostics notification");
    is($resp2->{method}, 'textDocument/publishDiagnostics', "method is publishDiagnostics");
    my $diag_params = $resp2->{params};
    is($diag_params->{uri}, 'file:///test.txt', "diagnostics for correct URI");
    ok(scalar @{$diag_params->{diagnostics}} > 0, "diagnostics array non-empty");
};

# ── Clean source has empty diagnostics ───────────────────────────────────────

subtest "didOpen with clean source has empty diagnostics" => sub {
    my $bridge = TestBridge->new();

    my $input_str = '';
    $input_str .= make_message({
        jsonrpc => '2.0',
        id      => 1,
        method  => 'initialize',
        params  => { processId => 1, capabilities => {} },
    });
    $input_str .= make_message({
        jsonrpc => '2.0',
        method  => 'textDocument/didOpen',
        params  => {
            textDocument => {
                uri     => 'file:///clean.txt',
                version => 1,
                text    => 'clean source code',
            },
        },
    });

    open my $in_fh,  '<', \$input_str  or die "Cannot open input: $!";
    my $output_str = '';
    open my $out_fh, '>', \$output_str or die "Cannot open output: $!";
    binmode($in_fh, ':raw');
    binmode($out_fh, ':raw');

    my $server = CodingAdventures::Ls00::Server->new($bridge, $in_fh, $out_fh);
    $server->serve();

    my $resp1 = read_message_from_string(\$output_str);
    my $resp2 = read_message_from_string(\$output_str);
    ok($resp2, "got diagnostics notification");
    is(scalar @{$resp2->{params}{diagnostics}}, 0, "no diagnostics for clean source");
};

# ── Shutdown request ─────────────────────────────────────────────────────────

subtest "shutdown returns null" => sub {
    my $bridge = TestBridge->new();

    my $input_str = '';
    $input_str .= make_message({
        jsonrpc => '2.0',
        id      => 1,
        method  => 'initialize',
        params  => { processId => 1, capabilities => {} },
    });
    $input_str .= make_message({
        jsonrpc => '2.0',
        id      => 2,
        method  => 'shutdown',
    });

    open my $in_fh,  '<', \$input_str  or die "Cannot open input: $!";
    my $output_str = '';
    open my $out_fh, '>', \$output_str or die "Cannot open output: $!";
    binmode($in_fh, ':raw');
    binmode($out_fh, ':raw');

    my $server = CodingAdventures::Ls00::Server->new($bridge, $in_fh, $out_fh);
    $server->serve();

    my $resp1 = read_message_from_string(\$output_str);
    my $resp2 = read_message_from_string(\$output_str);
    ok($resp2, "got shutdown response");
    is($resp2->{id}, 2, "shutdown response id = 2");
};

# ── Minimal bridge capabilities ──────────────────────────────────────────────

{
    package MinBridge;
    sub new { bless {}, shift }
    sub tokenize { return ([], undef) }
    sub parse    { return ($_[1], [], undef) }
}

subtest "minimal bridge capabilities" => sub {
    my $caps = build_capabilities(MinBridge->new());

    is($caps->{textDocumentSync}, 2, "textDocumentSync present");
    ok(!exists $caps->{hoverProvider}, "no hoverProvider");
    ok(!exists $caps->{definitionProvider}, "no definitionProvider");
};

done_testing();
