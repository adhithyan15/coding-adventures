use strict;
use warnings;
use Test2::V0;

use CodingAdventures::MosaicEmitWebcomponent;
use CodingAdventures::MosaicAnalyzer;

# ============================================================================
# Helpers
# ============================================================================

sub emit_ok {
    my ($src) = @_;
    my ($comp, $err) = CodingAdventures::MosaicAnalyzer->analyze($src);
    die "analyze: $err" if $err;
    return CodingAdventures::MosaicEmitWebcomponent->emit($comp);
}

# ============================================================================
# Output structure
# ============================================================================

subtest 'emit returns hashref with filename and content' => sub {
    my $r = emit_ok('component A { Box {} }');
    is(ref $r, 'HASH', 'returns hashref');
    ok(defined $r->{filename}, 'filename key');
    ok(defined $r->{content},  'content key');
};

subtest 'filename is ComponentName.ts' => sub {
    my $r = emit_ok('component MyCard { Box {} }');
    is($r->{filename}, 'MyCard.ts', 'filename is MyCard.ts');
};

subtest 'content has auto-generated header' => sub {
    my $r = emit_ok('component A { Box {} }');
    like($r->{content}, qr/AUTO-GENERATED/, 'has auto-generated comment');
};

# ============================================================================
# Custom Element class
# ============================================================================

subtest 'extends HTMLElement' => sub {
    my $r = emit_ok('component A { Box {} }');
    like($r->{content}, qr/extends HTMLElement/, 'extends HTMLElement');
};

subtest 'class name is component name' => sub {
    my $r = emit_ok('component ProfileCard { Box {} }');
    like($r->{content}, qr/class ProfileCard extends HTMLElement/, 'class name');
};

subtest 'customElements.define with kebab-case tag' => sub {
    my $r = emit_ok('component ProfileCard { Box {} }');
    like($r->{content}, qr/customElements\.define\('mosaic-profile-card'/, 'define call');
};

subtest 'uses shadow DOM' => sub {
    my $r = emit_ok('component A { Box {} }');
    like($r->{content}, qr/attachShadow/, 'shadow DOM');
};

# ============================================================================
# Slot properties
# ============================================================================

subtest 'private field declaration for slot' => sub {
    my $r = emit_ok('component A { slot title: text; Box {} }');
    like($r->{content}, qr/private _title/, 'private _title field');
};

subtest 'getter and setter for slot' => sub {
    my $r = emit_ok('component A { slot title: text; Box {} }');
    like($r->{content}, qr/get title\(\)/, 'getter');
    like($r->{content}, qr/set title\(v/, 'setter');
};

subtest 'observedAttributes includes text slots' => sub {
    my $r = emit_ok('component A { slot title: text; Box {} }');
    like($r->{content}, qr/observedAttributes/, 'observedAttributes');
    like($r->{content}, qr/'title'/, 'title in observed');
};

# ============================================================================
# Render method
# ============================================================================

subtest '_render method present' => sub {
    my $r = emit_ok('component A { Box {} }');
    like($r->{content}, qr/private _render\(\)/, '_render method');
};

subtest '_escapeHtml helper present' => sub {
    my $r = emit_ok('component A { Box {} }');
    like($r->{content}, qr/_escapeHtml/, '_escapeHtml helper');
};

subtest 'Text node content uses escapeHtml for slot ref' => sub {
    my $r = emit_ok('component A { slot title: text; Text { content: @title; } }');
    like($r->{content}, qr/_escapeHtml/, '_escapeHtml used for text content');
};

subtest 'slot projection uses named slot' => sub {
    my $r = emit_ok('component A { slot header: node; Column { @header; } }');
    like($r->{content}, qr/<slot name="header">/, 'named slot element');
};

done_testing;
