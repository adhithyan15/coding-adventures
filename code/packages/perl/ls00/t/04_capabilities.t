#!/usr/bin/env perl

# 04_capabilities.t -- Capabilities advertisement tests

use strict;
use warnings;
use Test::More;

use CodingAdventures::Ls00::Capabilities qw(:all);

# ── Mock Bridges ─────────────────────────────────────────────────────────────

{
    package MinimalBridge;
    sub new { bless {}, shift }
    sub tokenize { return ([], undef) }
    sub parse    { return ($_[1], [], undef) }
}

{
    package FullBridge;
    sub new { bless {}, shift }
    sub tokenize        { return ([], undef) }
    sub parse           { return ($_[1], [], undef) }
    sub hover           { return (undef, undef) }
    sub definition      { return (undef, undef) }
    sub references      { return ([], undef) }
    sub completion      { return ([], undef) }
    sub rename          { return (undef, undef) }
    sub document_symbols { return ([], undef) }
    sub folding_ranges  { return ([], undef) }
    sub signature_help  { return (undef, undef) }
    sub format          { return ([], undef) }
    sub semantic_tokens { return ([], undef) }
}

{
    package HoverOnlyBridge;
    sub new { bless {}, shift }
    sub tokenize { return ([], undef) }
    sub parse    { return ($_[1], [], undef) }
    sub hover    { return (undef, undef) }
    sub document_symbols { return ([], undef) }
}

# ── Minimal bridge capabilities ─────────────────────────────────────────────

subtest "minimal bridge: only textDocumentSync" => sub {
    my $caps = build_capabilities(MinimalBridge->new());

    is($caps->{textDocumentSync}, 2, "textDocumentSync is 2 (incremental)");

    my @optional = qw(
        hoverProvider definitionProvider referencesProvider
        completionProvider renameProvider documentSymbolProvider
        foldingRangeProvider signatureHelpProvider
        documentFormattingProvider semanticTokensProvider
    );
    for my $cap (@optional) {
        ok(!exists $caps->{$cap}, "minimal bridge should not advertise $cap");
    }
};

# ── Full bridge capabilities ────────────────────────────────────────────────

subtest "full bridge: all capabilities present" => sub {
    my $caps = build_capabilities(FullBridge->new());

    is($caps->{textDocumentSync}, 2, "textDocumentSync is 2");
    ok($caps->{hoverProvider}, "hoverProvider present");
    ok($caps->{definitionProvider}, "definitionProvider present");
    ok($caps->{referencesProvider}, "referencesProvider present");
    is(ref $caps->{completionProvider}, 'HASH', "completionProvider is hashref");
    ok($caps->{renameProvider}, "renameProvider present");
    ok($caps->{documentSymbolProvider}, "documentSymbolProvider present");
    ok($caps->{foldingRangeProvider}, "foldingRangeProvider present");
    is(ref $caps->{signatureHelpProvider}, 'HASH', "signatureHelpProvider is hashref");
    ok($caps->{documentFormattingProvider}, "documentFormattingProvider present");
    is(ref $caps->{semanticTokensProvider}, 'HASH', "semanticTokensProvider is hashref");
};

# ── Selective capabilities ──────────────────────────────────────────────────

subtest "hover-only bridge" => sub {
    my $caps = build_capabilities(HoverOnlyBridge->new());

    ok($caps->{hoverProvider}, "hoverProvider present");
    ok($caps->{documentSymbolProvider}, "documentSymbolProvider present");
    ok(!exists $caps->{definitionProvider}, "no definitionProvider");
    ok(!exists $caps->{completionProvider}, "no completionProvider");
};

# ── Completion trigger characters ────────────────────────────────────────────

subtest "completion trigger characters" => sub {
    my $caps = build_capabilities(FullBridge->new());
    my $cp = $caps->{completionProvider};
    is(ref $cp->{triggerCharacters}, 'ARRAY', "triggerCharacters is arrayref");
    ok(scalar @{$cp->{triggerCharacters}} > 0, "has trigger characters");
};

# ── Signature help trigger characters ────────────────────────────────────────

subtest "signature help trigger characters" => sub {
    my $caps = build_capabilities(FullBridge->new());
    my $shp = $caps->{signatureHelpProvider};
    is(ref $shp->{triggerCharacters}, 'ARRAY', "triggerCharacters is arrayref");
};

# ── Semantic tokens has legend ───────────────────────────────────────────────

subtest "semantic tokens provider has legend" => sub {
    my $caps = build_capabilities(FullBridge->new());
    my $stp = $caps->{semanticTokensProvider};
    is(ref $stp->{legend}, 'HASH', "legend present");
    is(ref $stp->{legend}{tokenTypes}, 'ARRAY', "tokenTypes is arrayref");
    is(ref $stp->{legend}{tokenModifiers}, 'ARRAY', "tokenModifiers is arrayref");
};

done_testing();
