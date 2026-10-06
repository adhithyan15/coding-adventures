package CodingAdventures::BuildTool::SourceInputRegistry;

# An installed, immutable projection of the neutral source-input authority.
# The JSON file is package data: production never locates code/specs fixtures.
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use Encode qw(encode);
use File::Basename qw(dirname);
use File::Spec ();
use JSON::PP ();
use CodingAdventures::BuildTool::TrackedArtifactUnicode17 ();

my ($BYTES, $REGISTRY, $DIGEST, %LANGUAGE, %GENERATED);

sub _load {
    return if defined $REGISTRY;
    my $file = File::Spec->catfile(dirname(__FILE__), 'language-source-input-registry.json');
    open my $fh, '<:raw', $file or die "SOURCE_REGISTRY_MISSING: $!";
    local $/;
    my $bytes = <$fh>;
    close $fh or die "SOURCE_REGISTRY_READ_FAILED: $!";
    my $registry = JSON::PP->new->utf8->decode($bytes);
    die "SOURCE_REGISTRY_INVALID: version\n" unless $registry->{schema_version} == 1;
    my $canonical = JSON::PP->new->utf8->canonical->encode($registry);
    my $digest = sha256_hex(
        "coding-adventures/build-tool-language-source-input-registry/v1\0"
        . pack('Q>', length($canonical)) . $canonical
    );
    %LANGUAGE = map { $_->{language} => $_ } @{ $registry->{languages} };
    %GENERATED = map { $_ => 1 } @{ $registry->{universal_inputs}{generated_directory_components} };
    ($BYTES, $REGISTRY, $DIGEST) = ($bytes, $registry, $digest);
}

sub bytes { _load(); return $BYTES }
sub digest { _load(); return $DIGEST }
sub generated_component { _load(); return !!$GENERATED{$_[1]} }

sub entry {
    my ($self, $language) = @_;
    _load();
    die "SOURCE_LANGUAGE_INVALID: unknown language\n"
        unless defined($language) && exists $LANGUAGE{$language};
    return $LANGUAGE{$language};
}

# A repository path grants an exact-package selector only if it encodes the
# claimed lane. Isolated package tests receive an empty root and global rules.
sub canonical_root {
    my ($self, $language, $root) = @_;
    $self->entry($language);       # reject unknown before any path examination
    return '' unless defined($root) && length($root);
    die "SOURCE_PACKAGE_ROOT_INVALID: unsafe path\n" unless _valid_path($root);
    if ($root =~ m{\Acode/(?:packages|programs)/([^/]+)/[^/]+(?:/[^/]+)*\z}) {
        die "SOURCE_PACKAGE_ROOT_INVALID: lane mismatch\n" unless $1 eq $language;
        return $root;
    }
    if ($language eq 'typescript' && $root =~ m{\Acode/sites/(?:blog|landing-page)\z}) {
        my $entry = $self->entry('typescript');
        return $root if grep { $_->{package_root} eq $root } @{ $entry->{package_exact_inputs} };
    }
    die "SOURCE_PACKAGE_ROOT_INVALID: unregistered root\n";
}

sub _valid_path {
    my ($path) = @_;
    return 0 if !defined($path) || ref($path) || length(encode('UTF-8', $path)) > 512
        || $path =~ m{\A/|\A[A-Za-z]:|\\|//|[<>:"|?*]|[\p{Cc}\p{Cf}]}
        || CodingAdventures::BuildTool::TrackedArtifactUnicode17::nfc($path) ne $path;
    my %reserved = map { $_ => 1 } (qw(CON PRN AUX NUL CONIN$ CONOUT$ CLOCK$),
        map { "COM$_" } 1..9, map { "LPT$_" } 1..9);
    for my $part (split m{/}, $path, -1) {
        return 0 if $part eq '' || $part eq '.' || $part eq '..' || $part =~ /[ .]\z/;
        my ($base) = split /\./, $part, 2;
        return 0 if $reserved{CodingAdventures::BuildTool::TrackedArtifactUnicode17::full_uppercase($base)};
    }
    return 1;
}

sub select {
    my ($self, $language, $root, $mode, $patterns, $candidates, $match_declared) = @_;
    my $entry = $self->entry($language);   # fail before candidate traversal
    $root = $self->canonical_root($language, $root);
    die "SOURCE_MODE_INVALID: unknown mode\n"
        unless defined($mode) && ($mode eq 'extension' || $mode eq 'declared_sources');
    die "SOURCE_CANDIDATES_INVALID: expected array\n" unless ref($candidates) eq 'ARRAY';
    die "SOURCE_HASH_LIMIT_EXCEEDED: candidates\n" if @$candidates > 100_000;
    my %seen;
    my %prefix_seen;
    my %inert;
    my %files;
    for my $candidate (@$candidates) {
        die "SOURCE_CANDIDATES_INVALID: record\n" unless ref($candidate) eq 'HASH';
        my $path = $candidate->{path};
        die "SOURCE_PATH_INVALID: unsafe path\n" unless _valid_path($path);
        my $identity = CodingAdventures::BuildTool::TrackedArtifactUnicode17::casefold($path);
        die "SOURCE_PATH_INVALID: portable alias\n"
            if exists($seen{$identity}) && $seen{$identity} ne $path;
        die "SOURCE_PATH_INVALID: duplicate\n" if exists($seen{$identity});
        $seen{$identity} = $path;
        my $prefix = '';
        for my $part (split m{/}, $path) {
            $prefix = length($prefix) ? "$prefix/$part" : $part;
            my $folded = CodingAdventures::BuildTool::TrackedArtifactUnicode17::casefold($prefix);
            die "SOURCE_PATH_INVALID: portable prefix alias\n"
                if exists($prefix_seen{$folded}) && $prefix_seen{$folded} ne $prefix;
            $prefix_seen{$folded} = $prefix;
        }
        my $kind = $candidate->{kind};
        die "SOURCE_CANDIDATES_INVALID: kind\n"
            unless defined($kind) && $kind =~ /\A(?:file|symlink|reparse_point)\z/;
        $inert{$path} = 1 if $kind ne 'file';
        $files{$path} = 1 if $kind eq 'file';
    }
    for my $path (keys %files) {
        my @parts = split m{/}, $path;
        my $prefix = '';
        for my $part (@parts[0 .. $#parts - 1]) {
            $prefix = length($prefix) ? "$prefix/$part" : $part;
            die "SOURCE_PATH_INVALID: file prefix\n" if $files{$prefix};
        }
    }
    my @selected;
    CANDIDATE: for my $candidate (@$candidates) {
        my $path = $candidate->{path};
        next unless $candidate->{kind} eq 'file';
        my @parts = split m{/}, $path;
        my $prefix = '';
        for my $part (@parts[0 .. $#parts - 1]) {
            $prefix = length($prefix) ? "$prefix/$part" : $part;
            next CANDIDATE if $GENERATED{$part} || $inert{$prefix};
        }
        my $selected = _selected($REGISTRY, $entry, $root, $path, $mode);
        $selected ||= $match_declared->($path) if !$selected && $mode eq 'declared_sources';
        next unless $selected;
        push @selected, $path;
        die "SOURCE_HASH_LIMIT_EXCEEDED: selected\n" if @selected > 50_000;
    }
    return sort { encode('UTF-8', $a) cmp encode('UTF-8', $b) } @selected;
}

sub _selected {
    my ($registry, $entry, $root, $path, $mode) = @_;
    my @parts = split m{/}, $path;
    my $base = $parts[-1];
    my $at_root = @parts == 1;
    my $universal = $registry->{universal_inputs};
    return 1 if grep { $_ eq $base } @{ $universal->{build_filenames} };
    return 1 if $at_root && grep { $_ eq $base } @{ $universal->{root_exact_basenames} };
    return 1 if $at_root && grep { $_ eq $base } @{ $entry->{root_exact_basenames} };
    return 1 if $at_root && grep { length($base) >= length($_) && substr($base, -length($_)) eq $_ }
        @{ $entry->{root_variable_suffixes} };
    return 1 if grep { $_ eq $path } @{ $entry->{root_exact_relative_paths} };
    for my $rule (@{ $entry->{package_exact_inputs} }) {
        return 1 if $root eq $rule->{package_root}
            && grep { $_ eq $path } @{ $rule->{paths} };
    }
    return 0 if $mode eq 'declared_sources';
    return 1 if grep { length($base) >= length($_) && substr($base, -length($_)) eq $_ }
        @{ $entry->{recursive_suffixes} };
    return 1 if grep { $_ eq $base } @{ $entry->{recursive_exact_basenames} };
    for my $rule (@{ $entry->{scoped_inputs} }) {
        next if $rule->{scope} eq 'root' && !$at_root;
        next if $rule->{scope} eq 'subtree'
            && index($path, $rule->{path_prefix} . '/') != 0;
        return 1 if grep { $_ eq $base } @{ $rule->{exact_basenames} };
        return 1 if grep { length($base) >= length($_) && substr($base, -length($_)) eq $_ }
            @{ $rule->{suffixes} };
    }
    return 0;
}

1;
