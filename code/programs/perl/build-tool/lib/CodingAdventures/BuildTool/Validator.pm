package CodingAdventures::BuildTool::Validator;

use strict;
use warnings;
use File::Spec ();

my %CI_MANAGED_TOOLCHAIN_LANGUAGES = map { $_ => 1 } qw(
    python ruby typescript rust elixir lua perl
);

sub validate_ci_full_build_toolchains {
    my ($root, $packages) = @_;

    my $ci_path = File::Spec->catfile($root, '.github', 'workflows', 'ci.yml');
    return undef unless -f $ci_path;

    open(my $fh, '<', $ci_path) or return undef;
    local $/;
    my $workflow = <$fh>;
    close($fh);

    return undef if index($workflow, 'Full build on main merge') < 0;

    (my $compact_workflow = $workflow) =~ s/\s+//g;
    my @langs = languages_needing_ci_toolchains($packages);
    my @missing_output_binding;
    my @missing_main_force;

    for my $lang (@langs) {
        my $output_binding = "needs_${lang}:\${{steps.toolchains.outputs.needs_${lang}}}";
        push @missing_output_binding, $lang
            if index($compact_workflow, $output_binding) < 0;

        my $force_binding = "needs_${lang}=true";
        push @missing_main_force, $lang
            if index($compact_workflow, $force_binding) < 0;
    }

    return undef if !@missing_output_binding && !@missing_main_force;

    my @parts;
    if (@missing_output_binding) {
        push @parts,
            'detect outputs for forced main full builds are not normalized through steps.toolchains for: '
            . join(', ', @missing_output_binding);
    }
    if (@missing_main_force) {
        push @parts,
            'forced main full-build path does not explicitly enable toolchains for: '
            . join(', ', @missing_main_force);
    }

    $ci_path =~ s{\\}{/}g;
    return $ci_path . ': ' . join('; ', @parts);
}

sub languages_needing_ci_toolchains {
    my ($packages) = @_;
    my %seen;
    my @langs;

    for my $pkg (@{$packages || []}) {
        my $lang = $pkg->{language};
        next unless $lang && $CI_MANAGED_TOOLCHAIN_LANGUAGES{$lang};
        next if $seen{$lang}++;
        push @langs, $lang;
    }

    return sort @langs;
}

1;
