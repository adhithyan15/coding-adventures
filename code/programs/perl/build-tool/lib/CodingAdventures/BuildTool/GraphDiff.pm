package CodingAdventures::BuildTool::GraphDiff;

use strict;
use warnings;
use utf8;

use Digest::SHA qw(sha256_hex);
use JSON::PP ();

use CodingAdventures::BuildTool::TrackedArtifactUnicode17 ();

our $VERSION = '0.1.0';

use constant MAX_PACKAGES        => 4_096;
use constant MAX_EDGES           => 16_384;
use constant MAX_SOURCE_GLOBS    => 256;
use constant MAX_PATH_SCALARS    => 512;
use constant MAX_PACKAGE_SCALARS => 240;
use constant MAX_MATCH_WORK       => 50_000_000;

my %BUILD_FRONT = map { $_ => 1 } qw(
    BUILD BUILD_windows BUILD_mac BUILD_linux BUILD_mac_and_linux
);
my %WINDOWS_RESERVED = map { $_ => 1 } (
    qw(CON PRN AUX NUL CONIN$ CONOUT$ CLOCK$),
    (map { "COM$_" } 1 .. 9),
    (map { "LPT$_" } 1 .. 9),
    (map { "COM$_" } qw(¹ ² ³)),
    (map { "LPT$_" } qw(¹ ² ³)),
);
my $BOUNDARY_DOMAIN =
    "coding-adventures/build-tool-repository-source-input-boundary/v1\0";

sub evaluate_graph {
    my ($options) = @_;
    my $graph = _validate_graph($options->{packages}, $options->{edges});
    my $levels = _levels($graph);
    return {edges => [], levels => [], error_code => 'GRAPH_CYCLE'}
        if !defined $levels;
    return {edges => $graph->{edges}, levels => $levels, error_code => ''};
}

sub evaluate_diff_selection {
    my ($input, %extra) = @_;
    my $options = $input->{options};
    my @names = map { $_->{name} } @{$options->{packages}};
    my $graph = _validate_graph(\@names, $options->{edges});
    _require(defined _levels($graph), 'DIFF_EDGE_CYCLE');

    my ($packages, $compiled) =
        _validate_diff_input($options, $input->{changed_paths}, $graph);
    my $boundary_consumers = _boundary_reverse_index(
        $options, $packages, $extra{boundary},
    );

    my $remaining = MAX_MATCH_WORK;
    for my $package (@{$options->{packages}}) {
        next if $package->{source_mode} ne 'strict_globs';
        my $pattern_factor = 0;
        $pattern_factor += _scalar_length($_) + 1
            for @{$package->{source_globs} // []};
        for my $path (@{$input->{changed_paths}}) {
            next if !_inside($path, $package->{rel_path});
            my $relative = _relative($path, $package->{rel_path});
            next if $BUILD_FRONT{_basename($relative)};
            my $path_factor = _scalar_length($relative) + 1;
            return _diff_error('DIFF_MATCH_LIMIT_EXCEEDED')
                if $pattern_factor && $pattern_factor > int($remaining / $path_factor);
            $remaining -= $pattern_factor * $path_factor;
        }
    }

    my %changed = map { $_ => 1 } @{$options->{forced_packages}};
    my $unknown = 0;
    for my $path (@{$input->{changed_paths}}) {
        my $consumers = $boundary_consumers->{$path} || {};
        $changed{$_} = 1 for keys %{$consumers};
        my $known = keys(%{$consumers}) ? 1 : 0;
        for my $package (@{$options->{packages}}) {
            next if !_inside($path, $package->{rel_path});
            $known = 1;
            my $relative = _relative($path, $package->{rel_path});
            my $matched = $package->{source_mode} eq 'package_prefix'
                || $BUILD_FRONT{_basename($relative)};
            if (!$matched) {
                my $globs = $package->{source_globs} // [];
                for my $index (0 .. $#{$globs}) {
                    if (_match_path(
                        $globs->[$index], $relative,
                        $compiled->{$package->{name}}[$index],
                    )) {
                        $matched = 1;
                        last;
                    }
                }
            }
            $changed{$package->{name}} = 1 if $matched;
        }
        $unknown = 1 if !$known;
    }

    if ($unknown) {
        return _diff_error('DIFF_UNKNOWN_PATH')
            if $options->{unknown_path_policy} eq 'error';
        %changed = map { $_ => 1 } keys %{$packages};
    }

    my $affected = _closure(\%changed, $graph->{dependents});
    my $prerequisites = _closure($affected, $graph->{prerequisites});
    delete $prerequisites->{$_} for keys %{$affected};
    return {
        changed_packages      => [sort keys %changed],
        affected_packages     => [sort keys %{$affected}],
        prerequisite_packages => [sort keys %{$prerequisites}],
        error_code            => '',
    };
}

sub _validate_graph {
    my ($packages, $edges) = @_;
    _require(ref($packages) eq 'ARRAY' && @{$packages} <= MAX_PACKAGES,
        'GRAPH_PACKAGE_LIMIT_EXCEEDED');
    _require(ref($edges) eq 'ARRAY' && @{$edges} <= MAX_EDGES,
        'GRAPH_EDGE_LIMIT_EXCEEDED');
    my %names;
    for my $name (@{$packages}) {
        _require(_valid_package_name($name), 'GRAPH_PACKAGE_INVALID');
        _require(!$names{$name}, 'GRAPH_PACKAGE_DUPLICATE');
        $names{$name} = 1;
    }
    my (%seen, %dependents, %prerequisites);
    $dependents{$_} = [] for keys %names;
    $prerequisites{$_} = [] for keys %names;
    my @canonical;
    for my $edge (@{$edges}) {
        _require(ref($edge) eq 'ARRAY' && @{$edge} == 2, 'GRAPH_EDGE_UNKNOWN');
        my ($prerequisite, $dependent) = @{$edge};
        _require($names{$prerequisite} && $names{$dependent}, 'GRAPH_EDGE_UNKNOWN');
        _require($prerequisite ne $dependent, 'GRAPH_EDGE_SELF');
        my $key = $prerequisite . "\0" . $dependent;
        _require(!$seen{$key}, 'GRAPH_EDGE_DUPLICATE');
        $seen{$key} = 1;
        push @{$dependents{$prerequisite}}, $dependent;
        push @{$prerequisites{$dependent}}, $prerequisite;
        push @canonical, [$prerequisite, $dependent];
    }
    @canonical = sort { $a->[0] cmp $b->[0] || $a->[1] cmp $b->[1] } @canonical;
    @{$dependents{$_}} = sort @{$dependents{$_}} for keys %dependents;
    @{$prerequisites{$_}} = sort @{$prerequisites{$_}} for keys %prerequisites;
    return {
        names => \%names, edges => \@canonical,
        dependents => \%dependents, prerequisites => \%prerequisites,
    };
}

sub _levels {
    my ($graph) = @_;
    my %indegree = map { $_ => 0 } keys %{$graph->{names}};
    $indegree{$_->[1]}++ for @{$graph->{edges}};
    my @ready = sort grep { $indegree{$_} == 0 } keys %indegree;
    my (@levels, $visited);
    $visited = 0;
    while (@ready) {
        my @level = @ready;
        push @levels, [@level];
        my @next;
        for my $name (@level) {
            $visited++;
            for my $dependent (@{$graph->{dependents}{$name}}) {
                $indegree{$dependent}--;
                push @next, $dependent if $indegree{$dependent} == 0;
            }
        }
        @ready = sort @next;
    }
    return if $visited != keys %{$graph->{names}};
    return \@levels;
}

sub _validate_diff_input {
    my ($options, $changed_paths, $graph) = @_;
    my (%packages, %compiled, @root_identities);
    for my $package (@{$options->{packages}}) {
        my ($name, $root) = @{$package}{qw(name rel_path)};
        _require(_valid_package_name($name), 'DIFF_PACKAGE_INVALID');
        _require(_valid_portable_path($root), 'DIFF_PATH_INVALID');
        my $identity = CodingAdventures::BuildTool::TrackedArtifactUnicode17::casefold(
            CodingAdventures::BuildTool::TrackedArtifactUnicode17::nfc($root),
        );
        for my $prior (@root_identities) {
            _require(
                $identity ne $prior
                    && index($identity, "$prior/") != 0
                    && index($prior, "$identity/") != 0,
                'DIFF_PATH_INVALID',
            );
        }
        push @root_identities, $identity;
        _require($package->{source_mode} eq 'package_prefix'
            || $package->{source_mode} eq 'strict_globs', 'DIFF_SOURCE_MODE_INVALID');
        my $globs = $package->{source_globs} // [];
        _require(ref($globs) eq 'ARRAY' && @{$globs} <= MAX_SOURCE_GLOBS,
            'DIFF_GLOB_INVALID');
        my %seen;
        for my $pattern (@{$globs}) {
            _require(!$seen{$pattern}++ && _valid_portable_glob($pattern),
                'DIFF_GLOB_INVALID');
            my $compiled_pattern;
            eval { $compiled_pattern = _compile_pattern($pattern); 1 }
                or die 'DIFF_GLOB_INVALID';
            push @{$compiled{$name}}, $compiled_pattern;
        }
        _require($package->{source_mode} eq 'strict_globs' || !@{$globs},
            'DIFF_GLOB_INVALID');
        _require(!exists $packages{$name}, 'DIFF_PACKAGE_DUPLICATE');
        $packages{$name} = $package;
    }
    _require(keys(%packages) == keys(%{$graph->{names}}), 'DIFF_PACKAGE_INVALID');
    for my $name (keys %packages) {
        _require($graph->{names}{$name}, 'DIFF_PACKAGE_INVALID');
    }
    _require($options->{unknown_path_policy} eq 'all'
        || $options->{unknown_path_policy} eq 'error', 'DIFF_POLICY_INVALID');
    my $forced = $options->{forced_packages};
    _require(ref($forced) eq 'ARRAY' && @{$forced} <= MAX_PACKAGES,
        'DIFF_FORCED_PACKAGE_INVALID');
    my %forced_seen;
    for my $name (@{$forced}) {
        _require(!$forced_seen{$name}++, 'DIFF_FORCED_PACKAGE_INVALID');
        _require($packages{$name}, 'DIFF_FORCED_PACKAGE_UNKNOWN');
    }
    _require(ref($changed_paths) eq 'ARRAY' && @{$changed_paths} <= MAX_PACKAGES,
        'DIFF_PATH_INVALID');
    my %path_seen;
    for my $path (@{$changed_paths}) {
        _require(!$path_seen{$path}++ && _valid_portable_path($path),
            'DIFF_PATH_INVALID');
    }
    return (\%packages, \%compiled);
}

sub _boundary_reverse_index {
    my ($options, $packages, $boundary) = @_;
    my $wanted = $options->{boundary_sha256} // '';
    if ($wanted eq '') {
        _require(!defined $boundary, 'DIFF_BOUNDARY_DIGEST_MISMATCH');
        return {};
    }
    _require($wanted =~ /\A[0-9a-f]{64}\z/, 'DIFF_BOUNDARY_DIGEST_MISMATCH');
    _require(defined($boundary) && _boundary_digest($boundary) eq $wanted,
        'DIFF_BOUNDARY_DIGEST_MISMATCH');
    my %reverse;
    for my $package (values %{$packages}) {
        for my $rule (@{$boundary->{boundaries}}) {
            next if !_applies($rule->{applies_to}, $package->{rel_path});
            $reverse{$_->{path}}{$package->{name}} = 1 for @{$rule->{inputs}};
        }
    }
    return \%reverse;
}

sub _boundary_digest {
    my ($boundary) = @_;
    my $document = {
        schema_version => $boundary->{schema_version},
        language_source_input_registry_sha256 =>
            $boundary->{language_source_input_registry_sha256},
        boundaries => [map {
            +{
                id => $_->{id},
                input_origin => $_->{input_origin},
                applies_to => {
                    exact_roots => [@{$_->{applies_to}{exact_roots}}],
                    descendant_roots => [@{$_->{applies_to}{descendant_roots}}],
                    excluded_roots => [@{$_->{applies_to}{excluded_roots}}],
                },
                inputs => [map {
                    my %input = (path => $_->{path}, role => $_->{role});
                    $input{generated_component} = $_->{generated_component}
                        if defined($_->{generated_component})
                            && $_->{generated_component} ne '';
                    \%input;
                } @{$_->{inputs}}],
                reason => $_->{reason},
                owner => $_->{owner},
            }
        } @{$boundary->{boundaries}}],
    };
    my $encoded = JSON::PP->new->canonical->utf8->encode($document);
    my $length = length($encoded);
    my $framed = $BOUNDARY_DOMAIN . pack('NN', int($length / 4_294_967_296),
        $length % 4_294_967_296) . $encoded;
    return sha256_hex($framed);
}

sub _closure {
    my ($seeds, $adjacency) = @_;
    my %result = %{$seeds};
    my @pending = keys %{$seeds};
    my $cursor = 0;
    while ($cursor < @pending) {
        my $name = $pending[$cursor++];
        for my $next (@{$adjacency->{$name}}) {
            next if $result{$next};
            $result{$next} = 1;
            push @pending, $next;
        }
    }
    return \%result;
}

sub _applies {
    my ($applies, $root) = @_;
    return 1 if grep { $_ eq $root } @{$applies->{exact_roots}};
    return 0 if grep { $_ eq $root } @{$applies->{excluded_roots}};
    return scalar grep { index($root, "$_/") == 0 } @{$applies->{descendant_roots}};
}

sub _inside {
    my ($path, $root) = @_;
    return $path eq $root || index($path, "$root/") == 0;
}

sub _relative {
    my ($path, $root) = @_;
    return '' if $path eq $root;
    return substr($path, length($root) + 1);
}

sub _basename {
    my ($path) = @_;
    my @parts = split m{/}, $path;
    return $parts[-1] // '';
}

sub _valid_package_name {
    my ($value) = @_;
    return defined($value) && _scalar_length($value) <= MAX_PACKAGE_SCALARS
        && $value =~ /\A[a-z0-9][a-z0-9._-]*(?:\/[a-z0-9][a-z0-9._-]*)+\z/;
}

sub _valid_portable_path {
    my ($value) = @_;
    return 0 if !defined($value) || $value eq ''
        || _scalar_length($value) > MAX_PATH_SCALARS
        || CodingAdventures::BuildTool::TrackedArtifactUnicode17::nfc($value) ne $value
        || $value =~ m{\A/|\A[A-Za-z]:|\\|//|[<>:"|?*]|[\x00-\x1f]};
    for my $segment (split m{/}, $value, -1) {
        return 0 if $segment eq '' || $segment eq '.' || $segment eq '..'
            || $segment =~ /[ .]\z/ || _reserved($segment);
    }
    return 1;
}

sub _valid_portable_glob {
    my ($value) = @_;
    return 0 if !defined($value) || $value eq ''
        || _scalar_length($value) > MAX_PATH_SCALARS
        || CodingAdventures::BuildTool::TrackedArtifactUnicode17::nfc($value) ne $value
        || $value =~ m{\A/|\A[A-Za-z]:|\\|//|[<>:"|?]|[\x00-\x1f]};
    for my $segment (split m{/}, $value, -1) {
        return 0 if $segment eq '' || $segment eq '.' || $segment eq '..'
            || $segment =~ /[ .]\z/;
        return 0 if $segment !~ /[*\[\]{}]/ && _reserved($segment);
    }
    return 1;
}

sub _reserved {
    my ($segment) = @_;
    my ($base) = split /\./, $segment, 2;
    my $upper = CodingAdventures::BuildTool::TrackedArtifactUnicode17::full_uppercase($base);
    return !!$WINDOWS_RESERVED{$upper};
}

# Compile the portable grammar without interpolating caller text into a regex.
# A globstar is represented by undef; other segments are arrays of tokens.
sub _compile_pattern {
    my ($pattern) = @_;
    my @compiled;
    for my $segment (grep { length($_) } split m{/}, $pattern) {
        push @compiled, $segment eq '**' ? undef : _parse_segment($segment);
    }
    return \@compiled;
}

sub _parse_segment {
    my ($segment) = @_;
    my @characters = split //u, $segment;
    my @next_closing;
    my $next;
    for (my $i = $#characters; $i >= 0; $i--) {
        $next = $i if $characters[$i] eq ']';
        $next_closing[$i] = $next;
    }
    my @tokens;
    my $index = 0;
    while ($index < @characters) {
        my $character = $characters[$index];
        if ($character eq '*') {
            push @tokens, {kind => 'star'}
                if !@tokens || $tokens[-1]{kind} ne 'star';
            $index++;
            next;
        }
        if ($character ne '[') {
            push @tokens, {kind => 'literal', value => $character};
            $index++;
            next;
        }
        my ($token, $after) = _parse_character_class(
            \@characters, $index, \@next_closing,
        );
        if (!defined $token) {
            push @tokens, {kind => 'literal', value => '['};
            $index++;
            next;
        }
        push @tokens, $token;
        $index = $after;
    }
    return \@tokens;
}

sub _parse_character_class {
    my ($characters, $opening, $next_closing) = @_;
    my $cursor = $opening + 1;
    my $negated = $cursor < @{$characters} && $characters->[$cursor] eq '!';
    $cursor++ if $negated;
    my $closing = $next_closing->[$cursor];
    $closing = $next_closing->[$cursor + 1]
        if defined($closing) && $closing == $cursor;
    return (undef, undef) if !defined $closing;
    my $body = join('', @{$characters}[$cursor .. $closing - 1]);
    die "invalid character class\n" if grep { index($body, $_) >= 0 }
        qw(-- && ~~ ||);
    my @members;
    my $member = $cursor;
    while ($member < $closing) {
        if ($member + 2 < $closing && $characters->[$member + 1] eq '-') {
            my ($start, $end) = @{$characters}[$member, $member + 2];
            die "invalid character class\n" if ord($start) > ord($end);
            push @members, [$start, $end];
            $member += 3;
        } else {
            push @members, $characters->[$member++];
        }
    }
    return ({kind => 'class', negated => $negated, members => \@members}, $closing + 1);
}

sub _match_path {
    my ($pattern, $path, $precompiled) = @_;
    my @path = grep { length($_) } split m{/}, $path;
    my $compiled = $precompiled // _compile_pattern($pattern);
    my @next = (0) x (@path + 1);
    $next[@path] = 1;
    for my $segment (reverse @{$compiled}) {
        my @row = (0) x (@path + 1);
        if (!defined $segment) {
            $row[@path] = $next[@path];
            for (my $i = $#path; $i >= 0; $i--) {
                $row[$i] = $next[$i] || $row[$i + 1];
            }
        } else {
            for (my $i = $#path; $i >= 0; $i--) {
                $row[$i] = $next[$i + 1] && _match_segment($segment, $path[$i]);
            }
        }
        @next = @row;
    }
    return !!$next[0];
}

sub _match_segment {
    my ($tokens, $value) = @_;
    if (!grep { $_->{kind} ne 'literal' } @{$tokens}) {
        return join('', map { $_->{value} } @{$tokens}) eq $value;
    }
    my @values = split //u, $value;
    my @next = (0) x (@values + 1);
    $next[@values] = 1;
    for my $token (reverse @{$tokens}) {
        my @row = (0) x (@values + 1);
        if ($token->{kind} eq 'star') {
            $row[@values] = $next[@values];
            for (my $i = $#values; $i >= 0; $i--) {
                $row[$i] = $next[$i] || $row[$i + 1];
            }
        } else {
            for (my $i = 0; $i < @values; $i++) {
                $row[$i] = $next[$i + 1] && _token_matches($token, $values[$i]);
            }
        }
        @next = @row;
    }
    return !!$next[0];
}

sub _token_matches {
    my ($token, $value) = @_;
    return $token->{value} eq $value if $token->{kind} eq 'literal';
    my $matched = 0;
    for my $member (@{$token->{members}}) {
        if (ref($member)) {
            $matched ||= ord($member->[0]) <= ord($value)
                && ord($value) <= ord($member->[1]);
        } else {
            $matched ||= $member eq $value;
        }
    }
    return $token->{negated} ? !$matched : !!$matched;
}

sub _scalar_length {
    my ($value) = @_;
    return scalar split //u, $value;
}

sub _diff_error {
    my ($code) = @_;
    return {
        changed_packages => [], affected_packages => [],
        prerequisite_packages => [], error_code => $code,
    };
}

sub _require {
    my ($condition, $code) = @_;
    die $code if !$condition;
}

1;
