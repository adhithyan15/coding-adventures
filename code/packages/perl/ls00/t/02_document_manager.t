#!/usr/bin/env perl

# 02_document_manager.t -- DocumentManager open/change/close tests

use strict;
use warnings;
use utf8;
use Test::More;

use CodingAdventures::Ls00::DocumentManager;

# ── Open and Get ─────────────────────────────────────────────────────────────

subtest "open and get" => sub {
    my $dm = CodingAdventures::Ls00::DocumentManager->new();
    $dm->open("file:///test.txt", "hello world", 1);

    my ($doc, $found) = $dm->get("file:///test.txt");
    ok($found, "document should be open");
    is($doc->{text}, "hello world", "text matches");
    is($doc->{version}, 1, "version is 1");
};

# ── Get missing document ────────────────────────────────────────────────────

subtest "get missing" => sub {
    my $dm = CodingAdventures::Ls00::DocumentManager->new();
    my ($doc, $found) = $dm->get("file:///nonexistent.txt");
    ok(!$found, "nonexistent document returns not found");
};

# ── Close ────────────────────────────────────────────────────────────────────

subtest "close" => sub {
    my $dm = CodingAdventures::Ls00::DocumentManager->new();
    $dm->open("file:///test.txt", "hello", 1);
    $dm->close("file:///test.txt");

    my ($doc, $found) = $dm->get("file:///test.txt");
    ok(!$found, "document should be gone after close");
};

# ── Full replacement ────────────────────────────────────────────────────────

subtest "apply changes: full replacement" => sub {
    my $dm = CodingAdventures::Ls00::DocumentManager->new();
    $dm->open("file:///test.txt", "hello world", 1);

    my $err = $dm->apply_changes("file:///test.txt", [
        { range => undef, new_text => "goodbye world" },
    ], 2);
    ok(!$err, "no error on full replacement");

    my ($doc, $found) = $dm->get("file:///test.txt");
    is($doc->{text}, "goodbye world", "text replaced");
    is($doc->{version}, 2, "version updated to 2");
};

# ── Incremental change ──────────────────────────────────────────────────────

subtest "apply changes: incremental" => sub {
    my $dm = CodingAdventures::Ls00::DocumentManager->new();
    $dm->open("file:///test.txt", "hello world", 1);

    my $err = $dm->apply_changes("file:///test.txt", [
        {
            range => {
                start => { line => 0, character => 6 },
                end   => { line => 0, character => 11 },
            },
            new_text => "Go",
        },
    ], 2);
    ok(!$err, "no error on incremental change");

    my ($doc, $found) = $dm->get("file:///test.txt");
    is($doc->{text}, "hello Go", "incremental change applied");
};

# ── Not open error ──────────────────────────────────────────────────────────

subtest "apply changes: not open" => sub {
    my $dm = CodingAdventures::Ls00::DocumentManager->new();
    my $err = $dm->apply_changes("file:///notopen.txt", [
        { range => undef, new_text => "x" },
    ], 1);
    ok($err, "error for applying changes to non-open document");
    like($err, qr/not open/, "error mentions not open");
};

# ── Incremental with emoji ──────────────────────────────────────────────────

subtest "incremental with emoji" => sub {
    my $dm = CodingAdventures::Ls00::DocumentManager->new();
    $dm->open("file:///test.txt", "A\x{1F3B8}B", 1);

    my $err = $dm->apply_changes("file:///test.txt", [
        {
            range => {
                start => { line => 0, character => 3 },
                end   => { line => 0, character => 4 },
            },
            new_text => "X",
        },
    ], 2);
    ok(!$err, "no error on emoji incremental change");

    my ($doc, $found) = $dm->get("file:///test.txt");
    is($doc->{text}, "A\x{1F3B8}X", "emoji change applied correctly");
};

# ── Multiple incremental changes ────────────────────────────────────────────

subtest "multiple incremental changes" => sub {
    my $dm = CodingAdventures::Ls00::DocumentManager->new();
    $dm->open("uri", "hello world", 1);

    my $err = $dm->apply_changes("uri", [
        {
            range => {
                start => { line => 0, character => 0 },
                end   => { line => 0, character => 5 },
            },
            new_text => "hi",
        },
    ], 2);
    ok(!$err, "first change applied");

    my ($doc, $found) = $dm->get("uri");
    is($doc->{text}, "hi world", "first change result correct");
};

done_testing();
