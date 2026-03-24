package CodingAdventures::BuildTool::Plan;

# Plan.pm -- Build Plan JSON Serialisation
# =========================================
#
# A "build plan" describes what would be built and in what order, without
# actually executing any commands. It is the output of `--dry-run` mode.
#
# Plan JSON format:
#
#   {
#     "groups": [
#       {
#         "level": 0,
#         "packages": [
#           {
#             "name": "perl/logic-gates",
#             "language": "perl",
#             "commands": ["cpanm --installdeps --quiet .", "prove -l -v t/"]
#           }
#         ]
#       },
#       {
#         "level": 1,
#         "packages": [...]
#       }
#     ],
#     "total_packages": 3
#   }
#
# Perl advantages demonstrated here:
#   - JSON::PP for serialisation with sorted keys (canonical output).
#   - Array of hashrefs — Perl's natural JSON-compatible data structure.

use strict;
use warnings;
use JSON::PP ();

our $VERSION = '0.01';

# new -- Constructor.
sub new {
    my ($class) = @_;
    return bless {}, $class;
}

# build -- Construct the plan data structure.
#
# @param \@packages  -- Packages to build (filtered by change detection).
# @param $graph      -- Dependency graph.
# @param \%pkg_map   -- name => package hashref.
# @return plan hashref ready for serialisation.
sub build {
    my ($self, $packages_ref, $graph, $pkg_map_ref) = @_;

    # Build a lookup for fast access.
    my %pkg_map;
    if ($pkg_map_ref) {
        %pkg_map = %{$pkg_map_ref};
    } else {
        %pkg_map = map { $_->{name} => $_ } @{$packages_ref};
    }

    my @groups;
    my $level = 0;

    for my $group ($graph->independent_groups()) {
        # Filter to packages in our build set.
        my @in_group = grep { exists $pkg_map{$_} } @{$group};
        next unless @in_group;

        my @pkg_entries = map {
            my $pkg = $pkg_map{$_};
            {
                name     => $_,
                language => $pkg->{language} // 'unknown',
                commands => $pkg->{build_commands} // [],
            }
        } @in_group;

        push @groups, {
            level    => $level,
            packages => \@pkg_entries,
        };

        $level++;
    }

    return {
        groups         => \@groups,
        total_packages => scalar @{$packages_ref},
    };
}

# to_json -- Serialise the plan to pretty-printed JSON.
#
# @param $plan_ref -- Plan hashref from build().
# @return JSON string.
sub to_json {
    my ($self, $plan_ref) = @_;
    return JSON::PP->new->pretty(1)->canonical(1)->encode($plan_ref);
}

# print_plan -- Print the plan in human-readable format to STDOUT.
#
# @param $plan_ref -- Plan hashref from build().
sub print_plan {
    my ($self, $plan_ref) = @_;

    printf "Build Plan (%d packages, %d levels)\n",
        $plan_ref->{total_packages},
        scalar @{ $plan_ref->{groups} };

    for my $group (@{ $plan_ref->{groups} }) {
        printf "\nLevel %d (parallel):\n", $group->{level};
        for my $pkg (@{ $group->{packages} }) {
            printf "  %s [%s]\n", $pkg->{name}, $pkg->{language};
            for my $cmd (@{ $pkg->{commands} }) {
                printf "    \$ %s\n", $cmd;
            }
        }
    }
}

1;

__END__

=head1 NAME

CodingAdventures::BuildTool::Plan - Build plan serialisation

=head1 SYNOPSIS

  use CodingAdventures::BuildTool::Plan;

  my $p = CodingAdventures::BuildTool::Plan->new();
  my $plan = $p->build(\@packages, $graph);
  $p->print_plan($plan);
  # or: print $p->to_json($plan);

=head1 DESCRIPTION

Builds and serialises a JSON description of what would be built and in what
order. Used by C<--dry-run> mode.

=cut
