use strict;
use warnings;
use Test2::V0;

use CodingAdventures::MosaicEmitReact;
use CodingAdventures::MosaicAnalyzer;

# ============================================================================
# Helpers
# ============================================================================

sub emit_ok {
    my ($src) = @_;
    my ($comp, $err) = CodingAdventures::MosaicAnalyzer->analyze($src);
    die "analyze: $err" if $err;
    return CodingAdventures::MosaicEmitReact->emit($comp);
}

# ============================================================================
# Output structure
# ============================================================================

subtest 'emit returns hashref with filename and content' => sub {
    my $r = emit_ok('component A { Box {} }');
    is(ref $r, 'HASH', 'returns hashref');
    ok(defined $r->{filename}, 'filename key exists');
    ok(defined $r->{content},  'content key exists');
};

subtest 'filename is ComponentName.tsx' => sub {
    my $r = emit_ok('component MyCard { Box {} }');
    is($r->{filename}, 'MyCard.tsx', 'filename is MyCard.tsx');
};

subtest 'content contains auto-generated header' => sub {
    my $r = emit_ok('component A { Box {} }');
    like($r->{content}, qr/AUTO-GENERATED/, 'has auto-generated comment');
};

subtest 'content imports React' => sub {
    my $r = emit_ok('component A { Box {} }');
    like($r->{content}, qr/import React from "react"/, 'imports React');
};

# ============================================================================
# Props interface
# ============================================================================

subtest 'props interface generated' => sub {
    my $r = emit_ok('component A { slot title: text; Box {} }');
    like($r->{content}, qr/interface AProps/, 'props interface present');
    like($r->{content}, qr/title.*string/,    'title: string in interface');
};

subtest 'optional slot has question mark' => sub {
    my $r = emit_ok('component A { slot count: number = 0; Box {} }');
    like($r->{content}, qr/count\?:.*number/, 'optional slot has ?');
};

subtest 'function component exported' => sub {
    my $r = emit_ok('component MyBtn { Box {} }');
    like($r->{content}, qr/export function MyBtn/, 'exported function');
};

# ============================================================================
# Node rendering
# ============================================================================

subtest 'Column maps to div' => sub {
    my $r = emit_ok('component A { Column {} }');
    like($r->{content}, qr/<div/, 'Column emits <div');
};

subtest 'Text maps to span' => sub {
    my $r = emit_ok('component A { Text {} }');
    like($r->{content}, qr/<span/, 'Text emits <span');
};

subtest 'Image maps to self-closing img' => sub {
    my $r = emit_ok('component A { Image {} }');
    like($r->{content}, qr/<img/, 'Image emits <img');
};

# ============================================================================
# Property rendering
# ============================================================================

subtest 'dimension property in style' => sub {
    my $r = emit_ok('component A { Box { padding: 16dp; } }');
    like($r->{content}, qr/padding.*16px/, 'padding: 16px in output');
};

subtest 'hex color in style as rgba' => sub {
    my $r = emit_ok('component A { Box { background: #fff; } }');
    like($r->{content}, qr/rgba\(255, 255, 255/, 'rgba color in output');
};

subtest 'slot ref in content' => sub {
    my $r = emit_ok('component A { slot title: text; Text { content: @title; } }');
    like($r->{content}, qr/\{title\}/, 'slot ref {title} in output');
};

done_testing;
