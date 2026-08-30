package CodingAdventures::BuildTool::ToolchainDetection;

use strict;
use warnings;
use bytes ();
use JSON::PP ();

# Pure, bounded CI toolchain detection over caller-supplied BUILD snapshots.
# This module deliberately does not read the filesystem or environment, launch
# processes, inspect Git, or access the network. Callers own every input byte.

my $MAX_BUILD_BYTES = 65_536;
my $MAX_BUILD_LINES = 4_096;
my $MAX_AGGREGATE_BUILD_BYTES = 1_048_576;
my $DECLARATION_PREFIX = '# needs-toolchain:';

my @CANONICAL_TOOLCHAINS = qw(
    cpp
    dart
    dotnet
    elixir
    go
    haskell
    java
    kotlin
    lua
    ocaml
    perl
    python
    ruby
    rust
    swift
    typescript
);
my %CANONICAL_TOOLCHAIN_SET = map { $_ => 1 } @CANONICAL_TOOLCHAINS;

sub canonical_toolchains {
    return [@CANONICAL_TOOLCHAINS];
}

sub _utf8_byte_length {
    my ($value) = @_;
    return bytes::length($value);
}

sub _logical_line_count {
    my ($content) = @_;
    my $count = 1;
    my $offset = 0;
    while (1) {
        my $newline = index($content, "\n", $offset);
        return $count if $newline < 0;
        ++$count;
        $offset = $newline + 1;
    }
}

sub _trim_ascii_space {
    my ($value) = @_;
    $value =~ s/\A[ \t]+//;
    $value =~ s/[ \t]+\z//;
    return $value;
}

sub parse_extra_toolchains {
    my ($content) = @_;
    return [] if _utf8_byte_length($content) > $MAX_BUILD_BYTES;
    return [] if _logical_line_count($content) > $MAX_BUILD_LINES;

    my @lines = split(/\n/, $content, -1);
    my @declarations;
    my %seen;
    for my $index (0 .. $#lines) {
        my $line = $lines[$index];
        if ($index < $#lines && substr($line, -1) eq "\r") {
            chop($line);
        }
        $line = _trim_ascii_space($line);
        next unless index($line, $DECLARATION_PREFIX) == 0;

        my $suffix = substr($line, length($DECLARATION_PREFIX));
        next unless $suffix =~ /\A[ \t]/;
        my $name = _trim_ascii_space($suffix);
        next unless $CANONICAL_TOOLCHAIN_SET{$name};
        next if $seen{$name}++;
        push @declarations, $name;
    }
    return \@declarations;
}

sub _validate_snapshot_limits {
    my ($packages) = @_;
    my $aggregate_bytes = 0;
    for my $package (@$packages) {
        for my $content (values %{$package->{build_files}}) {
            my $bytes = _utf8_byte_length($content);
            if ($bytes > $MAX_BUILD_BYTES || _logical_line_count($content) > $MAX_BUILD_LINES) {
                die "toolchain BUILD snapshot exceeds its per-file resource ceiling\n";
            }
            $aggregate_bytes += $bytes;
        }
    }
    if ($aggregate_bytes > $MAX_AGGREGATE_BUILD_BYTES) {
        die "toolchain BUILD snapshot exceeds its aggregate resource ceiling\n";
    }
}

sub _build_file_candidates {
    my ($platform) = @_;
    return [qw(BUILD_mac BUILD_mac_and_linux BUILD)] if $platform eq 'darwin';
    return [qw(BUILD_linux BUILD_mac_and_linux BUILD)] if $platform eq 'linux';
    return [qw(BUILD_windows BUILD)] if $platform eq 'windows' || $platform eq 'win32';
    die "unsupported target platform: $platform\n";
}

sub _selected_front {
    my ($build_files, $platform) = @_;
    for my $filename (@{_build_file_candidates($platform)}) {
        return $build_files->{$filename} if exists $build_files->{$filename};
    }
    return '';
}

sub _toolchain_for_language {
    my ($language) = @_;
    return 'rust' if $language eq 'wasm';
    return 'cpp' if $language eq 'c' || $language eq 'cpp';
    return 'dotnet' if $language eq 'csharp' || $language eq 'fsharp' || $language eq 'dotnet';
    return $language if $CANONICAL_TOOLCHAIN_SET{$language};
    return;
}

sub _unsupported {
    my ($package_name) = @_;
    my %diagnostic = (
        code => 'TOOLCHAIN_UNSUPPORTED',
        severity => 'error',
    );
    $diagnostic{package} = $package_name if defined $package_name;
    return {
        outcome => 'error',
        toolchains => {},
        diagnostics => [\%diagnostic],
    };
}

sub evaluate_snapshot {
    my ($platform, $force_full, $packages, $scheduled_packages, $forced_toolchains) = @_;
    _validate_snapshot_limits($packages);

    my @prepared = map {
        +{
            %$_,
            extra_toolchains => parse_extra_toolchains(
                _selected_front($_->{build_files}, $platform)
            ),
        }
    } @$packages;

    my $scheduled = defined($scheduled_packages)
        ? { map { $_ => 1 } @$scheduled_packages }
        : undef;
    my %toolchains = map {
        $_ => $force_full ? JSON::PP::true : JSON::PP::false
    } @CANONICAL_TOOLCHAINS;

    for my $package (@prepared) {
        next if defined($scheduled) && !$scheduled->{$package->{name}};
        my $toolchain = _toolchain_for_language($package->{language});
        return _unsupported($package->{name}) unless defined $toolchain;
        next if $force_full;

        $toolchains{$toolchain} = JSON::PP::true;
        $toolchains{$_} = JSON::PP::true for @{$package->{extra_toolchains}};
    }

    for my $forced (@{$forced_toolchains || []}) {
        return _unsupported(undef) unless $CANONICAL_TOOLCHAIN_SET{$forced};
        $toolchains{$forced} = JSON::PP::true;
    }

    return {
        outcome => 'ok',
        toolchains => \%toolchains,
        diagnostics => [],
    };
}

1;
