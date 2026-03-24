#!/usr/bin/env perl

# t/09-plan.t -- Tests for CodingAdventures::BuildTool::Plan
# ===========================================================
#
# Test plan serialisation.

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;
use JSON::PP;

use CodingAdventures::BuildTool::Plan;
use CodingAdventures::BuildTool::Resolver;

sub make_pkg {
    my (%args) = @_;
    return {
        name           => $args{name},
        path           => '/tmp',
        language       => $args{lang} // 'perl',
        build_commands => $args{cmds} // ['prove -l -v t/'],
    };
}

my $planner = CodingAdventures::BuildTool::Plan->new();
my $r       = CodingAdventures::BuildTool::Resolver->new();

# ---------------------------------------------------------------------------
# Test 1: Plan has correct structure
# ---------------------------------------------------------------------------
subtest 'plan has groups and total_packages' => sub {
    my @pkgs  = (make_pkg(name => 'perl/a'), make_pkg(name => 'perl/b'));
    my $graph = $r->resolve(\@pkgs);
    my %pmap  = map { $_->{name} => $_ } @pkgs;
    my $plan  = $planner->build(\@pkgs, $graph, \%pmap);

    ok(exists $plan->{groups},         'plan has groups key');
    ok(exists $plan->{total_packages}, 'plan has total_packages key');
    is($plan->{total_packages}, 2,     'total_packages is 2');
};

# ---------------------------------------------------------------------------
# Test 2: to_json produces valid JSON
# ---------------------------------------------------------------------------
subtest 'to_json produces valid JSON' => sub {
    my @pkgs  = (make_pkg(name => 'perl/a'));
    my $graph = $r->resolve(\@pkgs);
    my %pmap  = map { $_->{name} => $_ } @pkgs;
    my $plan  = $planner->build(\@pkgs, $graph, \%pmap);
    my $json  = $planner->to_json($plan);

    my $parsed = eval { JSON::PP::decode_json($json) };
    ok(!$@, "valid JSON: $@");
    is(ref $parsed, 'HASH', 'parsed as hash');
};

# ---------------------------------------------------------------------------
# Test 3: Groups contain package entries with name and language
# ---------------------------------------------------------------------------
subtest 'plan groups contain package entries' => sub {
    my @pkgs  = (make_pkg(name => 'perl/x'));
    my $graph = $r->resolve(\@pkgs);
    my %pmap  = map { $_->{name} => $_ } @pkgs;
    my $plan  = $planner->build(\@pkgs, $graph, \%pmap);

    my $first_group = $plan->{groups}[0];
    ok(defined $first_group, 'first group exists');

    my $pkg_entry = $first_group->{packages}[0];
    is($pkg_entry->{name},     'perl/x', 'package name correct');
    is($pkg_entry->{language}, 'perl',   'language correct');
};

done_testing();
