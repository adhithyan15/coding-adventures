use strict;
use warnings;
use Test2::V0;

use CodingAdventures::MosaicAnalyzer;

# ============================================================================
# Basic component analysis
# ============================================================================

subtest 'minimal component' => sub {
    my ($comp, $err) = CodingAdventures::MosaicAnalyzer->analyze(
        'component Empty { Box {} }'
    );
    is($err, undef, 'no error');
    ok(defined $comp, 'component defined');
    is($comp->{name}, 'Empty', 'component name is Empty');
    is(scalar @{ $comp->{slots} }, 0, 'no slots');
};

subtest 'component name extracted' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component ProfileCard { Box {} }'
    );
    is($comp->{name}, 'ProfileCard', 'name correct');
};

# ============================================================================
# Slot analysis
# ============================================================================

subtest 'text slot type' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { slot title: text; Box {} }'
    );
    is(scalar @{ $comp->{slots} }, 1, 'one slot');
    my $slot = $comp->{slots}[0];
    is($slot->{name}, 'title', 'slot name');
    is($slot->{type}{kind}, 'text', 'slot type kind');
    is($slot->{required}, 1, 'slot is required');
};

subtest 'number slot type' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { slot count: number; Box {} }'
    );
    is($comp->{slots}[0]{type}{kind}, 'number', 'number kind');
};

subtest 'bool slot type' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { slot visible: bool; Box {} }'
    );
    is($comp->{slots}[0]{type}{kind}, 'bool', 'bool kind');
};

subtest 'image slot type' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { slot avatar: image; Box {} }'
    );
    is($comp->{slots}[0]{type}{kind}, 'image', 'image kind');
};

subtest 'slot with default number' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { slot count: number = 0; Box {} }'
    );
    my $slot = $comp->{slots}[0];
    is($slot->{required}, 0, 'slot is optional');
    is($slot->{default_value}{kind}, 'number', 'default is number');
    is($slot->{default_value}{value}, 0, 'default value is 0');
};

subtest 'slot with boolean default' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { slot show: bool = true; Box {} }'
    );
    my $slot = $comp->{slots}[0];
    is($slot->{default_value}{kind}, 'bool', 'default is bool');
    is($slot->{default_value}{value}, 1, 'default is true');
};

subtest 'list<text> slot type' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { slot items: list<text>; Box {} }'
    );
    my $slot = $comp->{slots}[0];
    is($slot->{type}{kind}, 'list', 'slot type is list');
    is($slot->{type}{element_type}{kind}, 'text', 'element type is text');
};

# ============================================================================
# Node tree analysis
# ============================================================================

subtest 'root node tag' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { Column {} }'
    );
    is($comp->{tree}{tag}, 'Column', 'root tag is Column');
};

subtest 'primitive node detection' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { Text {} }'
    );
    is($comp->{tree}{is_primitive}, 1, 'Text is primitive');
};

subtest 'non-primitive (component) node detection' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { MyButton {} }'
    );
    is($comp->{tree}{is_primitive}, 0, 'MyButton is not primitive');
};

# ============================================================================
# Property analysis
# ============================================================================

subtest 'string property' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { Text { content: "hello"; } }'
    );
    my $props = $comp->{tree}{properties};
    is(scalar @$props, 1, 'one property');
    is($props->[0]{name}, 'content', 'prop name');
    is($props->[0]{value}{kind}, 'string', 'value kind');
    is($props->[0]{value}{value}, '"hello"', 'value');
};

subtest 'dimension property' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { Box { padding: 16dp; } }'
    );
    my $prop = $comp->{tree}{properties}[0];
    is($prop->{value}{kind}, 'dimension', 'dimension kind');
    is($prop->{value}{value}, 16, 'value 16');
    is($prop->{value}{unit}, 'dp', 'unit dp');
};

subtest 'hex color property' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { Box { background: #2563eb; } }'
    );
    my $prop = $comp->{tree}{properties}[0];
    is($prop->{value}{kind}, 'color_hex', 'color_hex kind');
    is($prop->{value}{value}, '#2563eb', 'hex value');
};

subtest 'slot_ref property value' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { slot title: text; Text { content: @title; } }'
    );
    my $prop = $comp->{tree}{properties}[0];
    is($prop->{value}{kind}, 'slot_ref', 'slot_ref kind');
    is($prop->{value}{slot_name}, 'title', 'slot name');
};

# ============================================================================
# Children
# ============================================================================

subtest 'child node' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { Column { Text {} } }'
    );
    my $children = $comp->{tree}{children};
    is(scalar @$children, 1, 'one child');
    is($children->[0]{kind}, 'node', 'child kind is node');
    is($children->[0]{node}{tag}, 'Text', 'child tag is Text');
};

subtest 'slot reference as child' => sub {
    my ($comp) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { slot header: node; Column { @header; } }'
    );
    my $children = $comp->{tree}{children};
    is(scalar @$children, 1, 'one child');
    is($children->[0]{kind}, 'slot_ref', 'child is slot_ref');
    is($children->[0]{slot_name}, 'header', 'slot name');
};

subtest 'when block' => sub {
    my ($comp, $err) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { slot show: bool; Column { when @show { Text {} } } }'
    );
    is($err, undef, 'no error');
    my $children = $comp->{tree}{children};
    is($children->[0]{kind}, 'when', 'when block');
    is($children->[0]{slot_name}, 'show', 'slot name');
};

subtest 'each block' => sub {
    my ($comp, $err) = CodingAdventures::MosaicAnalyzer->analyze(
        'component A { slot items: list<text>; Column { each @items as item { Text {} } } }'
    );
    is($err, undef, 'no error');
    my $children = $comp->{tree}{children};
    is($children->[0]{kind}, 'each', 'each block');
    is($children->[0]{slot_name}, 'items', 'slot name');
    is($children->[0]{item_name}, 'item', 'item name');
};

# ============================================================================
# Error cases
# ============================================================================

subtest 'parse error propagated' => sub {
    my ($comp, $err) = CodingAdventures::MosaicAnalyzer->analyze('not valid mosaic');
    ok(defined $err, 'error returned');
    is($comp, undef, 'no component');
};

done_testing;
