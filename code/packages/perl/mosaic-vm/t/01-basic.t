use strict;
use warnings;
use Test2::V0;

use CodingAdventures::MosaicVm;
use CodingAdventures::MosaicAnalyzer;

# ============================================================================
# A simple recording renderer for testing.
#
# Records every method call and its arguments so we can assert on the
# sequence of calls that the VM made.
# ============================================================================

package TestRenderer;

sub new { bless { log => [], emit_result => 'test-output' }, shift }

sub begin_component {
    my ($self, $name, $slots) = @_;
    push @{ $self->{log} }, { method => 'begin_component', name => $name };
}

sub end_component {
    my ($self) = @_;
    push @{ $self->{log} }, { method => 'end_component' };
}

sub begin_node {
    my ($self, $tag, $is_primitive, $props, $ctx) = @_;
    push @{ $self->{log} }, { method => 'begin_node', tag => $tag,
                               is_primitive => $is_primitive, props => $props };
}

sub end_node {
    my ($self, $tag) = @_;
    push @{ $self->{log} }, { method => 'end_node', tag => $tag };
}

sub render_slot_child {
    my ($self, $slot_name, $slot_type, $ctx) = @_;
    push @{ $self->{log} }, { method => 'render_slot_child', slot_name => $slot_name };
}

sub begin_when {
    my ($self, $slot_name, $ctx) = @_;
    push @{ $self->{log} }, { method => 'begin_when', slot_name => $slot_name };
}

sub end_when {
    my ($self) = @_;
    push @{ $self->{log} }, { method => 'end_when' };
}

sub begin_each {
    my ($self, $slot_name, $item_name, $element_type, $ctx) = @_;
    push @{ $self->{log} }, { method => 'begin_each', slot_name => $slot_name,
                               item_name => $item_name };
}

sub end_each {
    my ($self) = @_;
    push @{ $self->{log} }, { method => 'end_each' };
}

sub emit {
    my ($self) = @_;
    return $self->{emit_result};
}

package main;

# ============================================================================
# Helpers
# ============================================================================

sub analyze_ok {
    my ($src) = @_;
    my ($comp, $err) = CodingAdventures::MosaicAnalyzer->analyze($src);
    die "analyze error: $err" if $err;
    return $comp;
}

sub run_ok {
    my ($src) = @_;
    my $comp = analyze_ok($src);
    my $r = TestRenderer->new();
    my ($result, $err) = CodingAdventures::MosaicVm->run($comp, $r);
    die "vm error: $err" if $err;
    return ($result, $r->{log});
}

sub method_names {
    my ($log) = @_;
    [ map { $_->{method} } @$log ];
}

# ============================================================================
# Basic call sequence
# ============================================================================

subtest 'minimal component call sequence' => sub {
    my ($result, $log) = run_ok('component A { Box {} }');
    is(method_names($log), [qw(begin_component begin_node end_node end_component)],
        'correct call sequence');
};

subtest 'emit result is returned' => sub {
    my ($result, $log) = run_ok('component A { Box {} }');
    is($result, 'test-output', 'emit result returned');
};

subtest 'component name passed to begin_component' => sub {
    my ($result, $log) = run_ok('component ProfileCard { Box {} }');
    is($log->[0]{name}, 'ProfileCard', 'component name passed');
};

# ============================================================================
# Node traversal
# ============================================================================

subtest 'nested nodes' => sub {
    my ($result, $log) = run_ok('component A { Column { Text {} } }');
    is(method_names($log),
        [qw(begin_component begin_node begin_node end_node end_node end_component)],
        'nested node call sequence');
};

subtest 'begin/end_node pass correct tag' => sub {
    my ($result, $log) = run_ok('component A { Column {} }');
    my ($begin) = grep { $_->{method} eq 'begin_node' } @$log;
    my ($end)   = grep { $_->{method} eq 'end_node'   } @$log;
    is($begin->{tag}, 'Column', 'begin_node tag');
    is($end->{tag},   'Column', 'end_node tag');
};

subtest 'primitive node flag' => sub {
    my ($result, $log) = run_ok('component A { Text {} }');
    my ($begin) = grep { $_->{method} eq 'begin_node' } @$log;
    is($begin->{is_primitive}, 1, 'Text is primitive');
};

subtest 'non-primitive node flag' => sub {
    my ($result, $log) = run_ok('component A { MyButton {} }');
    my ($begin) = grep { $_->{method} eq 'begin_node' } @$log;
    is($begin->{is_primitive}, 0, 'MyButton is not primitive');
};

# ============================================================================
# Value resolution
# ============================================================================

subtest 'string property resolved' => sub {
    my ($result, $log) = run_ok('component A { Text { content: "hello"; } }');
    my ($begin) = grep { $_->{method} eq 'begin_node' } @$log;
    my $prop = $begin->{props}[0];
    is($prop->{value}{kind},  'string', 'string kind');
    is($prop->{value}{value}, '"hello"', 'string value');
};

subtest 'dimension property resolved' => sub {
    my ($result, $log) = run_ok('component A { Box { padding: 16dp; } }');
    my ($begin) = grep { $_->{method} eq 'begin_node' } @$log;
    my $prop = $begin->{props}[0];
    is($prop->{value}{kind},  'dimension', 'dimension kind');
    is($prop->{value}{value}, 16,          'dimension value');
    is($prop->{value}{unit},  'dp',        'dimension unit');
};

subtest 'hex color #rgb resolved to RGBA' => sub {
    my ($result, $log) = run_ok('component A { Box { background: #fff; } }');
    my ($begin) = grep { $_->{method} eq 'begin_node' } @$log;
    my $prop = $begin->{props}[0];
    is($prop->{value}{kind}, 'color', 'color kind');
    is($prop->{value}{r},    255,     'r=255');
    is($prop->{value}{g},    255,     'g=255');
    is($prop->{value}{b},    255,     'b=255');
    is($prop->{value}{a},    255,     'a=255');
};

subtest 'hex color #rrggbb resolved' => sub {
    my ($result, $log) = run_ok('component A { Box { background: #2563eb; } }');
    my ($begin) = grep { $_->{method} eq 'begin_node' } @$log;
    my $prop = $begin->{props}[0];
    is($prop->{value}{kind}, 'color', 'color kind');
    is($prop->{value}{r},    0x25,    'r correct');
    is($prop->{value}{g},    0x63,    'g correct');
    is($prop->{value}{b},    0xeb,    'b correct');
    is($prop->{value}{a},    255,     'a defaults to 255');
};

# ============================================================================
# when / each blocks
# ============================================================================

subtest 'when block calls begin_when / end_when' => sub {
    my ($result, $log) = run_ok(
        'component A { slot show: bool; Column { when @show { Text {} } } }'
    );
    ok(grep({ $_->{method} eq 'begin_when' } @$log), 'begin_when called');
    ok(grep({ $_->{method} eq 'end_when'   } @$log), 'end_when called');
};

subtest 'each block calls begin_each / end_each' => sub {
    my ($result, $log) = run_ok(
        'component A { slot items: list<text>; Column { each @items as item { Text {} } } }'
    );
    ok(grep({ $_->{method} eq 'begin_each' } @$log), 'begin_each called');
    ok(grep({ $_->{method} eq 'end_each'   } @$log), 'end_each called');
    my ($begin_each) = grep { $_->{method} eq 'begin_each' } @$log;
    is($begin_each->{slot_name}, 'items', 'slot_name passed to begin_each');
    is($begin_each->{item_name}, 'item',  'item_name passed to begin_each');
};

# ============================================================================
# Error cases
# ============================================================================

subtest 'missing renderer method returns error' => sub {
    my $comp = analyze_ok('component A { Box {} }');
    # A renderer with emit() missing
    my $bad_renderer = bless {}, 'BadRenderer';
    no strict 'refs';
    for my $m (qw(begin_component end_component begin_node end_node
                  render_slot_child begin_when end_when begin_each end_each)) {
        *{"BadRenderer::$m"} = sub {};
    }
    my ($result, $err) = CodingAdventures::MosaicVm->run($comp, $bad_renderer);
    ok(defined $err, 'error returned for missing emit');
};

done_testing;
