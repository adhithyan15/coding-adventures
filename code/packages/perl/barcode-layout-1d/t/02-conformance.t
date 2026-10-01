use strict;
use warnings;
use utf8;
use Test2::V0;
use Digest::SHA qw(sha256_hex);
use FindBin;
use JSON::PP;
use File::Spec;

use lib '../paint-instructions/lib';
require CodingAdventures::BarcodeLayout1D;
my $pkg = 'CodingAdventures::BarcodeLayout1D';

my $MAX_FIXTURE_BYTES = 131_072;

sub fixture_fail { die "fixture-load-error: $_[0]\n"; }

sub fixture_preflight {
    my ($encoded, $limit) = @_;
    fixture_fail('fixture-size-limit') if length($encoded) > $MAX_FIXTURE_BYTES;
    my @stack;
    my $index = 0;
    while ($index < length($encoded)) {
        my $char = substr($encoded, $index, 1);
        if ($char eq '"') {
            my $start = $index++;
            my $escaped = 0;
            while ($index < length($encoded)) {
                my $next = substr($encoded, $index++, 1);
                if ($escaped) { $escaped = 0; next; }
                if ($next eq '\\') { $escaped = 1; next; }
                last if $next eq '"';
            }
            if (@stack && $stack[-1]{type} eq 'object' && $stack[-1]{expect_key}) {
                my $token = substr($encoded, $start, $index - $start);
                my $key = eval { JSON::PP->new->utf8->decode($token) };
                fixture_fail('fixture-invalid-json') if $@;
                fixture_fail('fixture-duplicate-key') if $stack[-1]{keys}{$key}++;
                $stack[-1]{expect_key} = 0;
            }
            next;
        }
        if ($char eq '{') {
            push @stack, {type => 'object', expect_key => 1, keys => {}};
            fixture_fail('fixture-depth-limit') if @stack > $limit;
        } elsif ($char eq '[') {
            push @stack, {type => 'array'};
            fixture_fail('fixture-depth-limit') if @stack > $limit;
        } elsif ($char eq ',') {
            $stack[-1]{expect_key} = 1
                if @stack && $stack[-1]{type} eq 'object';
        } elsif ($char eq '}' || $char eq ']') {
            pop @stack;
        }
        $index++;
    }
}

sub fixture_tree {
    my ($value, $depth, $limit) = @_;
    fixture_fail('fixture-depth-limit') if $depth > $limit;
    if (!ref $value) {
        return unless defined $value;
        require B;
        my $flags = B::svref_2object(\$value)->FLAGS;
        if (($flags & B::SVp_POK()) && !($flags & (B::SVp_IOK() | B::SVp_NOK()))) {
            fixture_fail('fixture-invalid-scalar') unless utf8::valid($value);
            fixture_fail('fixture-invalid-scalar')
                if grep { $_ >= 0xD800 && $_ <= 0xDFFF } unpack 'U*', $value;
            return;
        }
        return if ($flags & B::SVp_IOK()) && !($flags & (B::SVp_POK() | B::SVp_NOK()));
        fixture_fail('fixture-invalid-type');
    }
    if (ref($value) eq 'ARRAY') {
        fixture_tree($_, $depth + 1, $limit) for @{$value};
        return;
    }
    if (ref($value) eq 'HASH') {
        for my $key (keys %{$value}) {
            fixture_tree($key, $depth + 1, $limit);
            fixture_tree($value->{$key}, $depth + 1, $limit);
        }
        return;
    }
    return if JSON::PP::is_bool($value);
    fixture_fail('fixture-invalid-type');
}

sub exact_keys {
    my ($value, @wanted) = @_;
    fixture_fail('fixture-schema-invalid') unless ref($value) eq 'HASH';
    my %wanted = map { $_ => 1 } @wanted;
    fixture_fail('fixture-schema-invalid')
        if grep { !$wanted{$_} } keys %{$value};
    fixture_fail('fixture-schema-invalid')
        if grep { !exists $value->{$_} } @wanted;
}

sub validate_document {
    my ($document) = @_;
    exact_keys($document, qw(schema_version profile limits error_ids cases));
    fixture_fail('fixture-schema-invalid')
        unless $document->{schema_version} == 1
            && $document->{profile} eq 'barcode-layout-1d-v1'
            && ref($document->{limits}) eq 'HASH'
            && ref($document->{error_ids}) eq 'ARRAY'
            && ref($document->{cases}) eq 'ARRAY'
            && @{$document->{cases}} >= 48 && @{$document->{cases}} <= 64;
    my %limits = (
        max_pattern_scalars => 65_567, max_runs => 40_979,
        max_content_modules => 65_567, max_quiet_zone_modules => 4_096,
        max_symbols => 40_979, max_label_scalars => 4_096,
        max_metadata_entries => 64, max_metadata_key_scalars => 128,
        max_metadata_value_scalars => 4_096, max_metadata_utf8_bytes => 65_536,
        max_module_width => 8_192, max_bar_height => 8_192,
        max_color_scalars => 128, max_cases => 64,
        max_fixture_bytes => 131_072, max_fixture_depth => 8,
    );
    exact_keys($document->{limits}, keys %limits);
    fixture_fail('fixture-schema-invalid')
        if grep { $document->{limits}{$_} != $limits{$_} } keys %limits;
    my @errors = qw(pattern-too-long empty-pattern invalid-binary-token
        invalid-width-token invalid-marker-configuration invalid-module-count
        too-many-runs content-too-wide non-alternating-runs invalid-quiet-zone
        too-many-symbols symbol-width-mismatch invalid-render-config
        metadata-too-large human-readable-text-unsupported invalid-source-attribution);
    fixture_fail('fixture-schema-invalid')
        unless join("\0", @{$document->{error_ids}}) eq join("\0", @errors);
    my %operations = map { $_ => 1 }
        qw(expand-binary expand-width compute-layout project-scene);
    my %seen;
    for my $case (@{$document->{cases}}) {
        exact_keys($case, qw(id operation input expected));
        fixture_fail('fixture-schema-invalid')
            unless $case->{id} =~ /\Alayout-v1-[a-z0-9]+(?:-[a-z0-9]+)*\z/
                && !$seen{$case->{id}}++ && $operations{$case->{operation}}
                && ref($case->{input}) eq 'HASH' && ref($case->{expected}) eq 'HASH';
        my @expected = keys %{$case->{expected}};
        fixture_fail('fixture-schema-invalid') unless @expected == 1;
        my %input_keys = map { $_ => 1 } qw(pattern repeat sourceLabel sourceIndex role
            narrowMarker wideMarker narrowModules wideModules startingColor runs
            repeatRuns quietZoneModules symbols repeatSymbols renderConfig label
            metadata humanReadableText);
        fixture_fail('fixture-schema-invalid')
            if grep { !$input_keys{$_} } keys %{$case->{input}};
        for my $entry ([repeat => 65_569], [repeatRuns => 40_980],
            [repeatSymbols => 40_980]) {
            next unless exists $case->{input}{$entry->[0]};
            my $repeat = $case->{input}{$entry->[0]};
            fixture_fail('fixture-schema-invalid')
                unless ref($repeat) eq 'HASH'
                    && CodingAdventures::BarcodeLayout1D::_v1_integer($repeat->{count})
                    && $repeat->{count} >= 0 && $repeat->{count} <= $entry->[1];
        }
        my %allowed = $case->{operation} =~ /\Aexpand-/
            ? map { $_ => 1 } qw(runs runDigest error)
            : $case->{operation} eq 'compute-layout'
                ? map { $_ => 1 } qw(layout error)
                : map { $_ => 1 } qw(scene error);
        fixture_fail('fixture-schema-invalid') unless $allowed{$expected[0]};
        fixture_fail('fixture-schema-invalid')
            if $expected[0] eq 'error' && ref($case->{expected}{error});
        fixture_fail('fixture-schema-invalid')
            if $expected[0] ne 'error'
                && ref($case->{expected}{$expected[0]}) ne
                    ($expected[0] eq 'runs' ? 'ARRAY' : 'HASH');
        my $input = $case->{input};
        if ($case->{operation} =~ /\Aexpand-/) {
            fixture_fail('fixture-schema-invalid')
                unless exists($input->{sourceLabel}) && exists($input->{sourceIndex})
                    && exists($input->{role})
                    && ((exists($input->{pattern}) ? 1 : 0)
                        + (exists($input->{repeat}) ? 1 : 0) == 1);
        } else {
            fixture_fail('fixture-schema-invalid')
                unless exists($input->{quietZoneModules})
                    && ((exists($input->{runs}) ? 1 : 0)
                        + (exists($input->{repeatRuns}) ? 1 : 0) == 1);
        }
    }
}

sub load_fixture_document {
    my ($schema_encoded, $document_encoded) = @_;
    fixture_preflight($schema_encoded, 24);
    fixture_preflight($document_encoded, 8);
    my $schema = eval { JSON::PP->new->utf8->decode($schema_encoded) };
    fixture_fail('fixture-invalid-json') if $@;
    my $document = eval { JSON::PP->new->utf8->decode($document_encoded) };
    fixture_fail('fixture-invalid-json') if $@;
    fixture_tree($schema, 0, 24);
    fixture_tree($document, 0, 8);
    fixture_fail('fixture-schema-invalid')
        unless ref($schema) eq 'HASH'
            && $schema->{'$id'} eq
                'https://coding-adventures.dev/schemas/barcode-layout-1d-v1.json';
    validate_document($document);
    return $document;
}

my $fixture_path = File::Spec->catfile(
    $FindBin::Bin, '..', '..', '..', '..', 'specs', 'fixtures',
    'barcode-layout-1d-v1', 'cases.json',
);
my $schema_path = File::Spec->catfile(
    $FindBin::Bin, '..', '..', '..', '..', 'specs', 'fixtures',
    'barcode-layout-1d-v1', 'schema.json',
);
open my $handle, '<:raw', $fixture_path or die "cannot open fixture: $!";
local $/;
my $encoded = <$handle>;
close $handle;
open my $schema_handle, '<:raw', $schema_path or die "cannot open schema: $!";
my $schema_encoded = <$schema_handle>;
close $schema_handle;
my $document = load_fixture_document($schema_encoded, $encoded);
ok(length($encoded) <= $MAX_FIXTURE_BYTES, 'fixture byte limit');
is(scalar @{$document->{cases}}, 56, 'all 56 cases loaded');

sub materialize_pattern {
    my ($input) = @_;
    return $input->{pattern} if exists $input->{pattern};
    my $repeat = $input->{repeat};
    fixture_fail('fixture-schema-invalid')
        unless CodingAdventures::BarcodeLayout1D::_v1_integer($repeat->{count})
            && $repeat->{count} >= 0 && $repeat->{count} <= 65_569;
    fixture_fail('fixture-schema-invalid')
        unless length($repeat->{token}) >= 1 && length($repeat->{token}) <= 2
            && length($repeat->{suffix} // q{}) <= 1;
    return ($repeat->{token} x $repeat->{count}) . ($repeat->{suffix} // q{});
}

sub materialize_runs {
    my ($input) = @_;
    return [map { +{%{$_}} } @{$input->{runs}}] if exists $input->{runs};
    my $repeat = $input->{repeatRuns};
    fixture_fail('fixture-schema-invalid')
        unless CodingAdventures::BarcodeLayout1D::_v1_integer($repeat->{count})
            && $repeat->{count} >= 0 && $repeat->{count} <= 40_980;
    return [map {
        +{
            color => $_ % 2 == 0 ? $repeat->{firstColor}
                : ($repeat->{firstColor} eq 'bar' ? 'space' : 'bar'),
            modules => $repeat->{modules}, sourceLabel => $repeat->{sourceLabel},
            sourceIndex => $repeat->{sourceIndex}, role => $repeat->{role},
        }
    } 0 .. $repeat->{count} - 1];
}

sub materialize_symbols {
    my ($input) = @_;
    return [map { +{%{$_}} } @{$input->{symbols}}] if exists $input->{symbols};
    return undef unless exists $input->{repeatSymbols};
    my $repeat = $input->{repeatSymbols};
    fixture_fail('fixture-schema-invalid')
        unless CodingAdventures::BarcodeLayout1D::_v1_integer($repeat->{count})
            && $repeat->{count} >= 0 && $repeat->{count} <= 40_980;
    return [map {
        +{label => $repeat->{label}, modules => $repeat->{modules},
          sourceIndex => $_, role => $repeat->{role}}
    } 0 .. $repeat->{count} - 1];
}

sub canonical_json {
    my ($value) = @_;
    return JSON::PP->new->utf8->canonical->encode($value);
}

sub scene_projection {
    my ($scene) = @_;
    return {
        width => $scene->{width}, height => $scene->{height},
        background => $scene->{background},
        rectangles => [map {
            +{x => $_->{x}, y => $_->{y}, width => $_->{width},
              height => $_->{height}, fill => $_->{fill},
              metadata => +{%{$_->{metadata}}}}
        } @{$scene->{instructions}}],
        metadata => +{%{$scene->{metadata}}},
    };
}

sub execute_case {
    my ($case) = @_;
    my $input = $case->{input};
    if ($case->{operation} eq 'expand-binary') {
        return $pkg->runs_from_binary_pattern_v1(
            materialize_pattern($input), source_label => $input->{sourceLabel},
            source_index => $input->{sourceIndex}, role => $input->{role},
        );
    }
    if ($case->{operation} eq 'expand-width') {
        return $pkg->runs_from_width_pattern_v1(
            materialize_pattern($input), source_label => $input->{sourceLabel},
            source_index => $input->{sourceIndex}, role => $input->{role},
            narrow_marker => $input->{narrowMarker} // 'N',
            wide_marker => $input->{wideMarker} // 'W',
            narrow_modules => $input->{narrowModules} // 1,
            wide_modules => $input->{wideModules} // 3,
            starting_color => $input->{startingColor} // 'bar',
        );
    }
    if ($case->{operation} eq 'compute-layout') {
        return $pkg->compute_barcode_1d_layout_v1(
            materialize_runs($input), $input->{quietZoneModules},
            materialize_symbols($input),
        );
    }
    if ($case->{operation} eq 'project-scene') {
        my $render = $input->{renderConfig} // {};
        return $pkg->project_barcode_1d_scene_v1(
            materialize_runs($input), $input->{quietZoneModules}, {
                render_config => {
                    module_width => $render->{moduleWidth} // 4,
                    bar_height => $render->{barHeight} // 120,
                    foreground => $render->{foreground} // '#000000',
                    background => $render->{background} // '#ffffff',
                    include_human_readable_text =>
                        $render->{includeHumanReadableText} // 0,
                },
                label => $input->{label} // '1D barcode',
                metadata => $input->{metadata} // {},
                human_readable_text => $input->{humanReadableText},
                symbols => materialize_symbols($input),
            },
        );
    }
    die "unsupported operation $case->{operation}";
}

my %counts;
for my $case (@{$document->{cases}}) {
    $counts{$case->{operation}}++;
    subtest $case->{id} => sub {
        my $expected = $case->{expected};
        if (exists $expected->{error}) {
            my $caught = dies { execute_case($case) };
            is($pkg->barcode_1d_error_id($caught), $expected->{error}, 'error ID');
            return;
        }
        my $actual = execute_case($case);
        if (exists $expected->{runs}) {
            is($actual, $expected->{runs}, 'runs');
        } elsif (exists $expected->{runDigest}) {
            my $digest = $expected->{runDigest};
            is(scalar @{$actual}, $digest->{runCount}, 'run count');
            my $content_modules = 0;
            $content_modules += $_->{modules} for @{$actual};
            is($content_modules, $digest->{contentModules}, 'content modules');
            is([@{$actual}[0, -1]], [$digest->{firstRun}, $digest->{lastRun}],
                'boundary runs');
            is(sha256_hex(canonical_json($actual)), $digest->{runsSha256}, 'run digest');
        } elsif (exists $expected->{layout}) {
            is($actual, $expected->{layout}, 'layout');
        } else {
            is(scene_projection($actual), $expected->{scene}, 'scene');
        }
    };
}
is(\%counts, { 'expand-binary' => 12, 'expand-width' => 12,
    'compute-layout' => 19, 'project-scene' => 13 }, 'operation counts');

subtest 'bounded loader rejects hostile envelopes before dispatch' => sub {
    like(dies { load_fixture_document($schema_encoded, '{"x":1,"x":2}') },
        qr/fixture-duplicate-key/);
    like(dies { load_fixture_document($schema_encoded, ('[' x 9) . '0' . (']' x 9)) },
        qr/fixture-depth-limit/);
    like(dies { load_fixture_document($schema_encoded, '[]') },
        qr/fixture-schema-invalid/);
    like(dies { load_fixture_document($schema_encoded, 'x' x 131_073) },
        qr/fixture-size-limit/);
    my $bad_scalar = $encoded;
    $bad_scalar =~ s/"layout-v1-binary-basic"/"\\ud800"/;
    like(dies { load_fixture_document($schema_encoded, $bad_scalar) },
        qr/fixture-(?:invalid-scalar|invalid-json)/);
};

subtest 'repeat forms are bounded before materialization' => sub {
    like(dies { materialize_pattern({repeat => {token => '1', count => 65_570}}) },
        qr/fixture-schema-invalid/);
    like(dies { materialize_runs({repeatRuns => {count => 40_981}}) },
        qr/fixture-schema-invalid/);
    like(dies { materialize_symbols({repeatSymbols => {count => 40_981}}) },
        qr/fixture-schema-invalid/);
};

subtest 'strict ingress distinguishes JSON strings and integers' => sub {
    is($pkg->barcode_1d_error_id(dies {
        $pkg->runs_from_binary_pattern_v1(101,
            source_label => 'A', source_index => 0, role => 'data');
    }), 'invalid-binary-token', 'numeric pattern is not a string');
    is($pkg->barcode_1d_error_id(dies {
        $pkg->runs_from_binary_pattern_v1('1',
            source_label => 65, source_index => 0, role => 'data');
    }), 'invalid-source-attribution', 'numeric label is not a string');
    is($pkg->barcode_1d_error_id(dies {
        $pkg->runs_from_binary_pattern_v1('1',
            source_label => 'A', source_index => '0', role => 'data');
    }), 'invalid-source-attribution', 'numeric string is not an integer');
    is($pkg->barcode_1d_error_id(dies {
        $pkg->project_barcode_1d_scene_v1(
            [{color => 'bar', modules => 1, sourceLabel => 'A',
              sourceIndex => 0, role => 'data'}], 1,
            {metadata => {caller => 1}},
        );
    }), 'metadata-too-large', 'numeric metadata value is not a string');
};

subtest 'text-value-fails-before-native-resolution' => sub {
    no warnings 'redefine';
    local *CodingAdventures::PaintInstructions::paint_rect =
        sub { die "native paint resolver reached\n" };
    local *CodingAdventures::PaintInstructions::paint_scene =
        sub { die "native paint resolver reached\n" };
    my $error = dies {
        $pkg->project_barcode_1d_scene_v1(
            [{color => 'bar', modules => 0, sourceLabel => 'A',
              sourceIndex => 0, role => 'data'}], 0,
            {human_readable_text => '123'},
        );
    };
    is($pkg->barcode_1d_error_id($error), 'human-readable-text-unsupported');
};

subtest 'text-enabled-fails-before-native-resolution' => sub {
    no warnings 'redefine';
    local *CodingAdventures::PaintInstructions::paint_rect =
        sub { die "native paint resolver reached\n" };
    local *CodingAdventures::PaintInstructions::paint_scene =
        sub { die "native paint resolver reached\n" };
    my $error = dies {
        $pkg->project_barcode_1d_scene_v1(
            [{color => 'bar', modules => 0, sourceLabel => 'A',
              sourceIndex => 0, role => 'data'}], 0,
            {render_config => {include_human_readable_text => 1, module_width => 0}},
        );
    };
    is($pkg->barcode_1d_error_id($error), 'human-readable-text-unsupported');
};

subtest 'fresh copy semantics' => sub {
    my $runs = [{color => 'bar', modules => 1, sourceLabel => 'A',
        sourceIndex => 0, role => 'data'}];
    my $metadata = {caller => 'value'};
    my $first = $pkg->project_barcode_1d_scene_v1($runs, 1, {metadata => $metadata});
    $first->{metadata}{caller} = 'changed';
    $first->{instructions}[0]{metadata}{sourceLabel} = 'changed';
    $metadata->{caller} = 'caller-changed';
    my $second = $pkg->project_barcode_1d_scene_v1(
        $runs, 1, {metadata => {caller => 'value'}});
    is($second->{metadata}{caller}, 'value');
    is($second->{instructions}[0]{metadata}{sourceLabel}, 'A');
};

done_testing;
