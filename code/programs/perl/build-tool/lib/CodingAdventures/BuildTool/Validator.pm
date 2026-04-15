package CodingAdventures::BuildTool::Validator;

use strict;
use warnings;
use File::Basename qw(basename);
use File::Spec ();

my %CI_MANAGED_TOOLCHAIN_LANGUAGES = map { $_ => 1 } qw(
    python ruby typescript rust elixir lua perl java kotlin haskell
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

sub validate_build_contracts {
    my ($root, $packages) = @_;

    my @errors;
    my $ci_error = validate_ci_full_build_toolchains($root, $packages);
    push @errors, $ci_error if defined $ci_error;
    push @errors, validate_lua_isolated_build_files($packages);
    push @errors, validate_perl_build_files($packages);

    return undef unless @errors;
    return join("\n  - ", @errors);
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

sub validate_lua_isolated_build_files {
    my ($packages) = @_;
    my @errors;

    for my $pkg (@{$packages || []}) {
        next unless ($pkg->{language} // '') eq 'lua';
        next unless defined $pkg->{path};

        my $self_rock = 'coding-adventures-' . basename($pkg->{path});
        $self_rock =~ s/_/-/g;
        my %build_lines;

        for my $build_path (lua_build_files($pkg->{path})) {
            my @lines = read_build_lines($build_path);
            $build_lines{basename($build_path)} = [@lines];
            next unless @lines;

            my $foreign_remove = first_foreign_lua_remove(\@lines, $self_rock);
            if (defined $foreign_remove) {
                (my $normalized = $build_path) =~ s{\\}{/}g;
                push @errors,
                    $normalized . ': Lua BUILD removes unrelated rock ' . $foreign_remove
                    . '; isolated package builds should only remove the package they are rebuilding';
            }

            my $state_machine_index = first_line_containing(\@lines, '../state_machine', '..\\state_machine');
            my $directed_graph_index = first_line_containing(\@lines, '../directed_graph', '..\\directed_graph');
            if (defined $state_machine_index && defined $directed_graph_index &&
                $state_machine_index < $directed_graph_index) {
                (my $normalized = $build_path) =~ s{\\}{/}g;
                push @errors,
                    $normalized . ': Lua BUILD installs state_machine before directed_graph; '
                    . 'isolated LuaRocks builds require directed_graph first';
            }

            if ((guarded_local_lua_install(\@lines) ||
                    (basename($build_path) eq 'BUILD_windows' && local_lua_sibling_install(\@lines))) &&
                !self_install_disables_deps(\@lines, $self_rock)) {
                (my $normalized = $build_path) =~ s{\\}{/}g;
                push @errors,
                    $normalized . ': Lua BUILD bootstraps sibling rocks but the final '
                    . 'self-install does not pass --deps-mode=none or --no-manifest';
            }
        }

        my @missing_windows_deps = missing_lua_sibling_installs(
            $build_lines{BUILD} || [],
            $build_lines{BUILD_windows} || [],
        );
        if (@missing_windows_deps) {
            my $build_path = File::Spec->catfile($pkg->{path}, 'BUILD_windows');
            $build_path =~ s{\\}{/}g;
            push @errors,
                $build_path . ': Lua BUILD_windows is missing sibling installs present in BUILD: '
                . join(', ', @missing_windows_deps);
        }
    }

    return @errors;
}

sub validate_perl_build_files {
    my ($packages) = @_;
    my @errors;

    for my $pkg (@{$packages || []}) {
        next unless ($pkg->{language} // '') eq 'perl';
        next unless defined $pkg->{path};

        for my $build_path (lua_build_files($pkg->{path})) {
            my @lines = read_build_lines($build_path);
            next unless grep {
                index($_, 'cpanm') >= 0
                    && index($_, 'Test2::V0') >= 0
                    && index($_, '--notest') < 0
            } @lines;

            (my $normalized = $build_path) =~ s{\\}{/}g;
            push @errors,
                $normalized . ': Perl BUILD bootstraps Test2::V0 without --notest; '
                . 'isolated Windows installs can fail while installing the test framework itself';
        }
    }

    return @errors;
}

sub lua_build_files {
    my ($pkg_path) = @_;
    opendir(my $dh, $pkg_path) or return ();
    my @files =
        sort
        map { File::Spec->catfile($pkg_path, $_) }
        grep { /^BUILD/ && -f File::Spec->catfile($pkg_path, $_) }
        readdir($dh);
    closedir($dh);
    return @files;
}

sub read_build_lines {
    my ($build_path) = @_;
    open(my $fh, '<', $build_path) or return ();
    my @lines;
    while (my $line = <$fh>) {
        $line =~ s/^\s+|\s+$//g;
        next if $line eq '' || $line =~ /^#/;
        push @lines, $line;
    }
    close($fh);
    return @lines;
}

sub first_foreign_lua_remove {
    my ($lines, $self_rock) = @_;
    for my $line (@{$lines || []}) {
        next unless $line =~ /\bluarocks remove --force ([^ \t]+)/;
        return $1 if $1 ne $self_rock;
    }
    return undef;
}

sub first_line_containing {
    my ($lines, @needles) = @_;
    for my $index (0 .. $#{$lines || []}) {
        my $line = $lines->[$index];
        for my $needle (@needles) {
            return $index if index($line, $needle) >= 0;
        }
    }
    return undef;
}

sub guarded_local_lua_install {
    my ($lines) = @_;
    for my $line (@{$lines || []}) {
        return 1
            if index($line, 'luarocks show ') >= 0
            && (index($line, '../') >= 0 || index($line, '..\\') >= 0);
    }
    return 0;
}

sub local_lua_sibling_install {
    my ($lines) = @_;
    my @dirs = lua_sibling_install_dirs($lines);
    return @dirs ? 1 : 0;
}

sub self_install_disables_deps {
    my ($lines, $self_rock) = @_;
    for my $line (@{$lines || []}) {
        next if index($line, 'luarocks make') < 0 || index($line, $self_rock) < 0;
        return 1
            if index($line, '--deps-mode=none') >= 0
            || index($line, '--deps-mode none') >= 0
            || index($line, '--no-manifest') >= 0;
    }
    return 0;
}

sub missing_lua_sibling_installs {
    my ($unix_lines, $windows_lines) = @_;
    my %windows_deps = map { $_ => 1 } lua_sibling_install_dirs($windows_lines);
    return grep { !$windows_deps{$_} } lua_sibling_install_dirs($unix_lines);
}

sub lua_sibling_install_dirs {
    my ($lines) = @_;
    my %seen;
    my @dirs;

    for my $line (@{$lines || []}) {
        next if index($line, 'luarocks make') < 0;
        next unless $line =~ /\bcd\s+([.][.][\\\/][^ \t\r\n&()]+)/;

        (my $dep = $1) =~ s{\\}{/}g;
        next if $seen{$dep}++;
        push @dirs, $dep;
    }

    return sort @dirs;
}

1;
