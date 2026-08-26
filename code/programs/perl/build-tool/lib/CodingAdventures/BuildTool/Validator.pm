package CodingAdventures::BuildTool::Validator;

use strict;
use warnings;
use utf8;
use File::Basename qw(basename);
use File::Spec ();
use JSON::PP ();
use CodingAdventures::BuildTool::TrackedArtifactUnicode17 ();

my %CI_MANAGED_TOOLCHAIN_LANGUAGES = map { $_ => 1 } qw(
    python ruby typescript rust elixir lua perl java kotlin haskell
);

my $TRACKED_ARTIFACT_COMPONENT_IDENTITY = 'node_modules';
my $TRACKED_ARTIFACT_REDACTED_PATH = 'repository';
my $ORPHAN_SCAN_ROOT = 'code';
my $ORPHAN_LEDGER_PATH = 'code/BUILD-EXEMPTIONS';
my @ORPHAN_BUILD_NAMES = qw(
    BUILD BUILD_windows BUILD_mac BUILD_linux BUILD_mac_and_linux
);
my %ORPHAN_BUILD_RANK = map { $ORPHAN_BUILD_NAMES[$_] => $_ }
    0 .. $#ORPHAN_BUILD_NAMES;
my %ORPHAN_SKIP_COMPONENTS = map { $_ => 1 } qw(
    .git target node_modules vendor .venv _build deps .build dist-newstyle .cargo
);
my %PYTHON_BLANK_CODEPOINTS = map { $_ => 1 } (
    0x0009 .. 0x000D,
    0x001C .. 0x0020,
    0x0085,
    0x00A0,
    0x1680,
    0x2000 .. 0x200A,
    0x2028,
    0x2029,
    0x202F,
    0x205F,
    0x3000,
);
our $TRACKED_ARTIFACT_UNICODE_VERSION =
    $CodingAdventures::BuildTool::TrackedArtifactUnicode17::UNICODE_VERSION;
my %WINDOWS_RESERVED_BASENAMES = map { $_ => 1 } (
    qw(CON PRN AUX NUL CONIN$ CONOUT$ CLOCK$),
    map({ "COM$_" } 1 .. 9),
    map({ "LPT$_" } 1 .. 9),
    map({ "COM$_" } qw(¹ ² ³)),
    map({ "LPT$_" } qw(¹ ² ³)),
);

# Validate caller-supplied inert records without reading a checkout, following
# links, consulting Git, launching a process, or inheriting host Unicode data.
sub validate_tracked_artifact_snapshot {
    my ($entries, $unicode_version) = @_;
    $unicode_version //= $TRACKED_ARTIFACT_UNICODE_VERSION;
    die "tracked artifact Unicode version must be $TRACKED_ARTIFACT_UNICODE_VERSION\n"
        unless $unicode_version eq $TRACKED_ARTIFACT_UNICODE_VERSION;

    my @diagnostics;
    for my $entry (@{$entries || []}) {
        my ($normalized_path, $problem) = _normalize_tracked_artifact_path($entry->{path});
        my %details = (
            ordinal => $entry->{ordinal},
            entry_kind => $entry->{entry_kind},
        );
        if (defined $problem) {
            $details{problem} = $problem;
            push @diagnostics, {
                code => 'TRACKED_ARTIFACT_PATH_INVALID',
                severity => 'error',
                path => $TRACKED_ARTIFACT_REDACTED_PATH,
                details => \%details,
            };
            next;
        }

        my $forbidden = 0;
        for my $component (split m{/}, $normalized_path, -1) {
            if (CodingAdventures::BuildTool::TrackedArtifactUnicode17::nfkc_casefold($component)
                    eq $TRACKED_ARTIFACT_COMPONENT_IDENTITY) {
                $forbidden = 1;
                last;
            }
        }
        next unless $forbidden;
        push @diagnostics, {
            code => 'TRACKED_ARTIFACT_FORBIDDEN',
            severity => 'error',
            path => $normalized_path,
            details => \%details,
        };
    }

    @diagnostics = sort {
        $a->{code} cmp $b->{code}
            || $a->{path} cmp $b->{path}
            || _canonical_tracked_details($a->{details}) cmp _canonical_tracked_details($b->{details})
    } @diagnostics;
    return \@diagnostics;
}

sub _normalize_tracked_artifact_path {
    my ($path) = @_;
    my $normalized = $path // '';
    return (undef, 'EMPTY') unless length($normalized);
    return (undef, 'TOO_LONG') if length($normalized) > 512;
    $normalized =~ s{\\}{/}g;
    my @scalars = unpack('U*', $normalized);
    return (undef, 'TOO_LONG') if @scalars > 512;
    return (undef, 'NON_NFC')
        unless CodingAdventures::BuildTool::TrackedArtifactUnicode17::nfc($normalized)
            eq $normalized;
    return (undef, 'ABSOLUTE') if substr($normalized, 0, 1) eq '/';
    return (undef, 'DRIVE_QUALIFIED') if $normalized =~ /^[A-Za-z]:/;

    my @segments = split m{/}, $normalized, -1;
    return (undef, 'EMPTY_SEGMENT') if grep { $_ eq '' } @segments;
    return (undef, 'UNSAFE_CHARACTER') if grep {
        $_ < 32 || $_ == 0x3C || $_ == 0x3E || $_ == 0x3A ||
            $_ == 0x22 || $_ == 0x7C || $_ == 0x3F || $_ == 0x2A
    } @scalars;

    for my $segment (@segments) {
        return (undef, 'DOT_SEGMENT') if $segment eq '.' || $segment eq '..';
        return (undef, 'TRAILING_DOT_OR_SPACE') if $segment =~ /[. ]\z/;

        my ($basename) = split /\./, $segment, 2;
        my $uppercase =
            CodingAdventures::BuildTool::TrackedArtifactUnicode17::full_uppercase($basename);
        return (undef, 'RESERVED_BASENAME') if $WINDOWS_RESERVED_BASENAMES{$uppercase};
    }
    return ($normalized, undef);
}

sub _canonical_tracked_details {
    my ($details) = @_;
    return join("\0", map { defined($_) ? $_ : '' } (
        $details->{entry_kind},
        $details->{ordinal},
        $details->{problem},
    ));
}

# Validate a closed Cargo/BUILD/ledger snapshot without touching the host.
# Discovery belongs to the native Go front door. This adapter accepts only
# inert hashes and arrays and deliberately gains no filesystem, Git, process,
# environment, network, credential, or link-following authority.
sub validate_orphan_crate_snapshot {
    my ($snapshot) = @_;

    my @manifests = grep {
        !_orphan_artifact_path($_->{path})
    } @{$snapshot->{manifests} || []};
    my %directories = map { $_ => 1 } @{$snapshot->{directories} || []};
    my %manifest_by_path = map { $_->{path} => $_ } @manifests;
    my (%coverage, %empty_builds);
    for my $manifest (@manifests) {
        my $path = $manifest->{path};
        $coverage{$path} = _covering_orphan_build(
            $snapshot->{build_files} || [],
            $path,
            'runnable',
        );
        $empty_builds{$path} = _covering_orphan_build(
            $snapshot->{build_files} || [],
            $path,
            'empty',
        );
    }

    my @diagnostics;
    my %seen_exemption_paths;
    my @valid_exemptions;

    # Reserve portable identities before field-policy precedence. An invalid
    # first spelling must not let a later full-fold alias escape detection.
    for my $exemption (@{$snapshot->{exemptions} || []}) {
        my $path = $exemption->{path};
        my ($identity, $path_problem);
        if (_portable_orphan_path($path)) {
            $identity = _orphan_path_identity($path);
            if (!_under_orphan_scan_root($path)) {
                $path_problem = 'PATH_OUTSIDE_SCAN';
            }
            elsif (_orphan_artifact_path($path)) {
                $path_problem = 'PATH_ARTIFACT';
            }
        }
        else {
            $path_problem = 'PATH_UNSAFE';
        }

        my $duplicate = defined($identity) && $seen_exemption_paths{$identity};
        $seen_exemption_paths{$identity} = 1
            if defined($identity) && !$duplicate;

        my $kind = $exemption->{kind};
        my $problem;
        if (!defined($kind) || ($kind ne 'EXCLUDED' && $kind ne 'PENDING')) {
            $problem = 'UNKNOWN_KIND';
        }
        elsif (_python_blank($exemption->{reason})) {
            $problem = 'REASON_MISSING';
        }
        elsif ($duplicate) {
            $problem = 'DUPLICATE_PATH';
        }
        else {
            $problem = $path_problem;
        }

        if (defined $problem) {
            push @diagnostics, {
                code => 'ORPHAN_EXEMPTION_INVALID',
                severity => 'error',
                path => $ORPHAN_LEDGER_PATH,
                details => {
                    line => $exemption->{line},
                    problem => $problem,
                },
            };
            next;
        }
        push @valid_exemptions, $exemption;
    }

    my %active_exemptions;
    my $pending_exemption_count = 0;
    for my $exemption (@valid_exemptions) {
        my $path = $exemption->{path};
        my $stale_problem;
        if (!$directories{$path}) {
            $stale_problem = 'MISSING_DIRECTORY';
        }
        elsif (!$manifest_by_path{$path}) {
            $stale_problem = 'NO_MANIFEST';
        }
        elsif (defined $coverage{$path}) {
            $stale_problem = 'COVERED';
        }

        if (defined $stale_problem) {
            push @diagnostics, {
                code => 'ORPHAN_EXEMPTION_STALE',
                severity => 'error',
                path => $ORPHAN_LEDGER_PATH,
                details => {
                    entry_path => $path,
                    kind => $exemption->{kind},
                    line => $exemption->{line},
                    problem => $stale_problem,
                },
            };
            next;
        }

        $active_exemptions{$path} = $exemption;
        $pending_exemption_count++ if $exemption->{kind} eq 'PENDING';
    }

    for my $manifest (@manifests) {
        my $path = $manifest->{path};
        next if defined($coverage{$path}) || $active_exemptions{$path};

        if (defined $empty_builds{$path}) {
            push @diagnostics, {
                code => 'ORPHAN_CRATE_EMPTY_BUILD',
                severity => 'error',
                path => $path,
                details => {
                    build_path => $empty_builds{$path}{path},
                    manifest_kind => $manifest->{kind},
                },
            };
        }
        else {
            push @diagnostics, {
                code => 'ORPHAN_CRATE_UNLISTED',
                severity => 'error',
                path => $path,
                details => { manifest_kind => $manifest->{kind} },
            };
        }
    }

    @diagnostics = sort {
        _unicode_scalar_cmp($a->{code}, $b->{code})
            || _unicode_scalar_cmp($a->{path}, $b->{path})
            || _canonical_orphan_details($a->{details})
                cmp _canonical_orphan_details($b->{details})
    } @diagnostics;

    my %seen_codes;
    my @diagnostic_codes = sort grep { !$seen_codes{$_}++ }
        map { $_->{code} } @diagnostics;
    return {
        valid => @diagnostics ? JSON::PP::false : JSON::PP::true,
        diagnostic_codes => \@diagnostic_codes,
        pending_exemption_count => $pending_exemption_count,
        diagnostics => \@diagnostics,
    };
}

sub _covering_orphan_build {
    my ($build_files, $manifest_path, $wanted_state) = @_;
    my ($best, $best_parent, $best_rank);
    for my $build_file (@{$build_files || []}) {
        next unless defined($build_file->{state})
            && $build_file->{state} eq $wanted_state;
        my $path = $build_file->{path} // '';
        next unless $path =~ m{\A(.+)/([^/]+)\z};
        my ($parent, $name) = ($1, $2);
        next unless exists $ORPHAN_BUILD_RANK{$name};
        next unless _under_orphan_scan_root($parent);
        next unless $manifest_path eq $parent
            || index($manifest_path, "$parent/") == 0;

        my $rank = $ORPHAN_BUILD_RANK{$name};
        if (!defined($best)
            || _orphan_path_depth($parent) > _orphan_path_depth($best_parent)
            || (_orphan_path_depth($parent) == _orphan_path_depth($best_parent)
                && $rank < $best_rank)
            || (_orphan_path_depth($parent) == _orphan_path_depth($best_parent)
                && $rank == $best_rank
                && _unicode_scalar_cmp($path, $best->{path}) < 0)) {
            $best = $build_file;
            $best_parent = $parent;
            $best_rank = $rank;
        }
    }
    return $best;
}

sub _portable_orphan_path {
    my ($path) = @_;
    return 0 unless _valid_unicode_text($path);
    return 0 if $path eq '' || length($path) > 2048;
    my @scalars = unpack('U*', $path);
    return 0 if @scalars > 512;
    return 0
        unless CodingAdventures::BuildTool::TrackedArtifactUnicode17::nfc($path)
            eq $path;
    return 0 if substr($path, 0, 1) eq '/'
        || index($path, '\\') >= 0
        || index($path, '//') >= 0
        || $path =~ /^[A-Za-z]:/;
    return 0 if grep {
        $_ < 32 || $_ == 0x3C || $_ == 0x3E || $_ == 0x3A
            || $_ == 0x22 || $_ == 0x7C || $_ == 0x3F || $_ == 0x2A
    } @scalars;

    for my $component (split m{/}, $path, -1) {
        return 0 if $component eq '' || $component eq '.' || $component eq '..';
        return 0 if $component =~ /[. ]\z/;
        my ($basename) = split /\./, $component, 2;
        my $uppercase =
            CodingAdventures::BuildTool::TrackedArtifactUnicode17::full_uppercase($basename);
        return 0 if $WINDOWS_RESERVED_BASENAMES{$uppercase};
    }
    return 1;
}

sub _orphan_path_identity {
    my ($path) = @_;
    return CodingAdventures::BuildTool::TrackedArtifactUnicode17::casefold(
        CodingAdventures::BuildTool::TrackedArtifactUnicode17::nfc($path),
    );
}

sub _under_orphan_scan_root {
    my ($path) = @_;
    return defined($path)
        && ($path eq $ORPHAN_SCAN_ROOT || index($path, "$ORPHAN_SCAN_ROOT/") == 0);
}

sub _orphan_artifact_path {
    my ($path) = @_;
    return 0 unless defined($path) && !ref($path);
    return scalar grep { $ORPHAN_SKIP_COMPONENTS{$_} }
        split m{/}, $path, -1;
}

sub _python_blank {
    my ($value) = @_;
    return 0 unless _valid_unicode_text($value);
    pos($value) = 0;
    while ($value =~ /\G(.)/gcs) {
        return 0 unless $PYTHON_BLANK_CODEPOINTS{ord($1)};
    }
    return 1;
}

sub _valid_unicode_text {
    my ($value) = @_;
    return 0 unless defined($value) && !ref($value);
    return utf8::valid($value) if utf8::is_utf8($value);
    return $value !~ /[\x80-\xFF]/;
}

sub _orphan_path_depth {
    my ($path) = @_;
    return scalar split m{/}, $path, -1;
}

sub _unicode_scalar_cmp {
    my ($left, $right) = @_;
    my @left = unpack('U*', $left // '');
    my @right = unpack('U*', $right // '');
    my $limit = @left < @right ? scalar(@left) : scalar(@right);
    for my $index (0 .. $limit - 1) {
        my $comparison = $left[$index] <=> $right[$index];
        return $comparison if $comparison;
    }
    return @left <=> @right;
}

sub _canonical_orphan_details {
    my ($details) = @_;
    return JSON::PP->new->canonical(1)->ascii(1)->encode($details);
}

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
