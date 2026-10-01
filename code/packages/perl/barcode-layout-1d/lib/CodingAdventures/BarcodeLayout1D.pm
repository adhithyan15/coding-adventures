package CodingAdventures::BarcodeLayout1D;

use strict;
use warnings;
use Carp qw(croak);

use CodingAdventures::PaintInstructions ();

our $VERSION = '0.01';

my %DEFAULT_LAYOUT_CONFIG = (
    module_unit        => 4,
    bar_height         => 120,
    quiet_zone_modules => 10,
);

my %DEFAULT_PAINT_OPTIONS = (
    fill       => '#000000',
    background => '#ffffff',
    metadata   => {},
);

sub default_layout_config { return { %DEFAULT_LAYOUT_CONFIG }; }
sub default_paint_options { return { %DEFAULT_PAINT_OPTIONS }; }

sub _copy_metadata {
    my ($metadata) = @_;
    return {} unless defined $metadata;
    return { %{$metadata} };
}

sub _validate_layout_config {
    my ($config) = @_;
    croak 'module_unit must be a positive integer' unless $config->{module_unit} > 0;
    croak 'bar_height must be a positive integer' unless $config->{bar_height} > 0;
    croak 'quiet_zone_modules must be zero or a positive integer' if $config->{quiet_zone_modules} < 0;
}

sub _validate_run {
    my ($run) = @_;
    croak q(run color must be 'bar' or 'space') unless $run->{color} eq 'bar' || $run->{color} eq 'space';
    croak 'run modules must be a positive integer' unless $run->{modules} > 0;
}

sub runs_from_binary_pattern {
    my ($class, $pattern, %opts) = @_;
    return [] if !defined($pattern) || $pattern eq q{};

    my $bar_char = $opts{bar_char} // '1';
    my $space_char = $opts{space_char} // '0';
    my @tokens = split //, $pattern;
    my $current = $tokens[0];
    my $count = 1;
    my @runs;

    my $flush = sub {
        my ($token, $modules) = @_;
        my $color;
        if ($token eq $bar_char) {
            $color = 'bar';
        } elsif ($token eq $space_char) {
            $color = 'space';
        } else {
            croak sprintf('binary pattern contains unsupported token: "%s"', $token);
        }

        push @runs, {
            color        => $color,
            modules      => $modules,
            source_char  => $opts{source_char} // q{},
            source_index => $opts{source_index} // 0,
            role         => 'data',
            metadata     => _copy_metadata($opts{metadata}),
        };
    };

    for my $index (1 .. $#tokens) {
        if ($tokens[$index] eq $current) {
            $count++;
        } else {
            $flush->($current, $count);
            $current = $tokens[$index];
            $count = 1;
        }
    }
    $flush->($current, $count);
    return \@runs;
}

sub runs_from_width_pattern {
    my ($class, $pattern, $colors, %opts) = @_;
    croak 'pattern length must match colors length' unless length($pattern) == scalar(@{$colors});

    my $narrow_modules = $opts{narrow_modules} // 1;
    my $wide_modules = $opts{wide_modules} // 3;
    croak 'narrow_modules and wide_modules must be positive integers'
        unless $narrow_modules > 0 && $wide_modules > 0;

    my @runs;
    my @tokens = split //, $pattern;
    for my $index (0 .. $#tokens) {
        my $token = $tokens[$index];
        croak sprintf('width pattern contains unsupported token: "%s"', $token)
            unless $token eq 'N' || $token eq 'W';
        push @runs, {
            color        => $colors->[$index],
            modules      => ($token eq 'W') ? $wide_modules : $narrow_modules,
            source_char  => $opts{source_char},
            source_index => $opts{source_index},
            role         => $opts{role} // 'data',
            metadata     => _copy_metadata($opts{metadata}),
        };
    }
    return \@runs;
}

sub layout_barcode_1d {
    my ($class, $runs, $config, $options) = @_;
    $config  = { %DEFAULT_LAYOUT_CONFIG, %{ $config  // {} } };
    $options = { %DEFAULT_PAINT_OPTIONS, %{ $options // {} } };

    _validate_layout_config($config);

    my $quiet_zone_width = $config->{quiet_zone_modules} * $config->{module_unit};
    my $cursor_x = $quiet_zone_width;
    my @instructions;

    for my $run (@{$runs}) {
        _validate_run($run);
        my $width = $run->{modules} * $config->{module_unit};
        if ($run->{color} eq 'bar') {
            my $metadata = _copy_metadata($run->{metadata});
            $metadata->{source_char} = $run->{source_char};
            $metadata->{source_index} = $run->{source_index};
            $metadata->{modules} = $run->{modules};
            $metadata->{role} = $run->{role};
            push @instructions, CodingAdventures::PaintInstructions->paint_rect(
                $cursor_x,
                0,
                $width,
                $config->{bar_height},
                $options->{fill},
                $metadata,
            );
        }
        $cursor_x += $width;
    }

    my $metadata = _copy_metadata($options->{metadata});
    $metadata->{content_width} = $cursor_x - $quiet_zone_width;
    $metadata->{quiet_zone_width} = $quiet_zone_width;
    $metadata->{module_unit} = $config->{module_unit};
    $metadata->{bar_height} = $config->{bar_height};

    return CodingAdventures::PaintInstructions->paint_scene(
        $cursor_x + $quiet_zone_width,
        $config->{bar_height},
        \@instructions,
        $options->{background},
        $metadata,
    );
}

sub draw_one_dimensional_barcode {
    my ($class, @args) = @_;
    return $class->layout_barcode_1d(@args);
}

# Strict language-neutral v1 adapter.  The historical entry points above stay
# intact because the symbology packages use their legacy calling conventions.
my $MAX_PATTERN_SCALARS = 65_567;
my $MAX_RUNS = 40_979;
my $MAX_CONTENT_MODULES = 65_567;
my $MAX_QUIET_ZONE_MODULES = 4_096;
my $MAX_SYMBOLS = 40_979;
my $MAX_LABEL_SCALARS = 4_096;
my $MAX_METADATA_ENTRIES = 64;
my $MAX_METADATA_KEY_SCALARS = 128;
my $MAX_METADATA_VALUE_SCALARS = 4_096;
my $MAX_METADATA_UTF8_BYTES = 65_536;
my $MAX_MODULE_WIDTH = 8_192;
my $MAX_BAR_HEIGHT = 8_192;
my $MAX_COLOR_SCALARS = 128;
my %V1_ROLE = map { $_ => 1 } qw(data start stop guard check inter-character-gap);
my %V1_SYMBOL_ROLE = map { $_ => 1 } qw(data start stop guard check);

sub _v1_fail { croak $_[0]; }

sub _v1_scalar_flags {
    require B;
    return B::svref_2object(\$_[0])->FLAGS;
}

sub _v1_string {
    my ($value) = @_;
    return 0 unless defined $value && !ref $value;
    my $flags = _v1_scalar_flags($value);
    return ($flags & B::SVp_POK())
        && !($flags & (B::SVp_IOK() | B::SVp_NOK()));
}

sub barcode_1d_error_id {
    my ($class, $error) = @_;
    return q{} unless defined $error;
    return $1 if $error =~ /\A([a-z]+(?:-[a-z]+)+)/;
    return q{};
}

sub _v1_scalars {
    my ($value, $error_id) = @_;
    _v1_fail($error_id) unless _v1_string($value) && utf8::valid($value);
    my @points = unpack 'U*', $value;
    for my $point (@points) {
        _v1_fail($error_id) if $point >= 0xD800 && $point <= 0xDFFF;
    }
    return scalar @points;
}

sub _v1_integer {
    my ($value) = @_;
    return 0 unless defined $value && !ref $value;
    my $flags = _v1_scalar_flags($value);
    return ($flags & B::SVp_IOK())
        && !($flags & (B::SVp_POK() | B::SVp_NOK()));
}

sub _v1_source {
    my ($label, $index) = @_;
    _v1_fail('invalid-source-attribution')
        if _v1_scalars($label, 'invalid-source-attribution') > $MAX_LABEL_SCALARS;
    _v1_fail('invalid-source-attribution')
        unless _v1_integer($index) && $index >= -2_147_483_648 && $index <= 2_147_483_647;
}

sub runs_from_binary_pattern_v1 {
    my ($class, $pattern, %opts) = @_;
    my $length = _v1_scalars($pattern, 'invalid-binary-token');
    _v1_fail('pattern-too-long') if $length > $MAX_PATTERN_SCALARS;
    _v1_fail('empty-pattern') if $length == 0;
    my @tokens = unpack 'U*', $pattern;
    _v1_fail('invalid-binary-token') if grep { $_ != ord('0') && $_ != ord('1') } @tokens;
    _v1_source($opts{source_label}, $opts{source_index});
    _v1_fail('invalid-source-attribution') unless $V1_ROLE{$opts{role} // q{}};

    my @runs;
    my $current = shift @tokens;
    my $count = 1;
    for my $token (@tokens) {
        if ($token == $current) {
            $count++;
            next;
        }
        _v1_fail('too-many-runs') if @runs >= $MAX_RUNS;
        push @runs, {
            color => $current == ord('1') ? 'bar' : 'space', modules => $count,
            sourceLabel => $opts{source_label}, sourceIndex => 0 + $opts{source_index},
            role => $opts{role},
        };
        $current = $token;
        $count = 1;
    }
    _v1_fail('too-many-runs') if @runs >= $MAX_RUNS;
    push @runs, {
        color => $current == ord('1') ? 'bar' : 'space', modules => $count,
        sourceLabel => $opts{source_label}, sourceIndex => 0 + $opts{source_index},
        role => $opts{role},
    };
    return \@runs;
}

sub runs_from_width_pattern_v1 {
    my ($class, $pattern, %opts) = @_;
    my $length = _v1_scalars($pattern, 'invalid-width-token');
    _v1_fail('pattern-too-long') if $length > $MAX_PATTERN_SCALARS;
    _v1_fail('empty-pattern') if $length == 0;
    my $narrow_marker = $opts{narrow_marker} // 'N';
    my $wide_marker = $opts{wide_marker} // 'W';
    _v1_fail('invalid-marker-configuration')
        unless _v1_scalars($narrow_marker, 'invalid-marker-configuration') == 1
            && _v1_scalars($wide_marker, 'invalid-marker-configuration') == 1
            && $narrow_marker ne $wide_marker;
    my @tokens = split //u, $pattern;
    _v1_fail('invalid-width-token')
        if grep { $_ ne $narrow_marker && $_ ne $wide_marker } @tokens;
    _v1_source($opts{source_label}, $opts{source_index});
    _v1_fail('invalid-source-attribution') unless $V1_ROLE{$opts{role} // q{}};
    my $narrow = $opts{narrow_modules} // 1;
    my $wide = $opts{wide_modules} // 3;
    _v1_fail('invalid-module-count')
        unless _v1_integer($narrow) && _v1_integer($wide) && $narrow > 0 && $wide > 0;
    my $starting = $opts{starting_color} // 'bar';
    _v1_fail('invalid-marker-configuration') unless $starting eq 'bar' || $starting eq 'space';
    _v1_fail('too-many-runs') if $length > $MAX_RUNS;
    my @runs;
    my $content = 0;
    for my $index (0 .. $#tokens) {
        my $modules = $tokens[$index] eq $wide_marker ? $wide : $narrow;
        _v1_fail('content-too-wide') if $modules > $MAX_CONTENT_MODULES - $content;
        $content += $modules;
        my $color = $index % 2 == 0 ? $starting : ($starting eq 'bar' ? 'space' : 'bar');
        push @runs, {
            color => $color, modules => 0 + $modules,
            sourceLabel => $opts{source_label}, sourceIndex => 0 + $opts{source_index},
            role => $opts{role},
        };
    }
    return \@runs;
}

sub compute_barcode_1d_layout_v1 {
    my ($class, $runs, $quiet, $symbols) = @_;
    _v1_fail('too-many-runs') if @{$runs} > $MAX_RUNS;
    my ($content, $previous) = (0, undef);
    for my $run (@{$runs}) {
        _v1_fail('invalid-source-attribution')
            unless ($run->{color} // q{}) =~ /\A(?:bar|space)\z/
                && $V1_ROLE{$run->{role} // q{}};
        _v1_fail('invalid-module-count')
            unless _v1_integer($run->{modules}) && $run->{modules} > 0;
        _v1_source($run->{sourceLabel}, $run->{sourceIndex});
        _v1_fail('content-too-wide') if $run->{modules} > $MAX_CONTENT_MODULES - $content;
        $content += $run->{modules};
        _v1_fail('non-alternating-runs')
            if defined $previous && $previous eq $run->{color};
        $previous = $run->{color};
    }
    _v1_fail('invalid-quiet-zone')
        unless _v1_integer($quiet) && $quiet >= 1 && $quiet <= $MAX_QUIET_ZONE_MODULES;

    my @layouts;
    if (defined $symbols) {
        _v1_fail('too-many-symbols') if @{$symbols} > $MAX_SYMBOLS;
        my $cursor = 0;
        for my $symbol (@{$symbols}) {
            _v1_fail('invalid-module-count')
                unless _v1_integer($symbol->{modules}) && $symbol->{modules} > 0;
            _v1_source($symbol->{label}, $symbol->{sourceIndex});
            _v1_fail('invalid-source-attribution') unless $V1_SYMBOL_ROLE{$symbol->{role} // q{}};
            _v1_fail('symbol-width-mismatch')
                if $symbol->{modules} > $MAX_CONTENT_MODULES - $cursor;
            my $end = $cursor + $symbol->{modules};
            push @layouts, {
                label => $symbol->{label}, startModule => $cursor, endModule => $end,
                sourceIndex => 0 + $symbol->{sourceIndex}, role => $symbol->{role},
            };
            $cursor = $end;
        }
        _v1_fail('symbol-width-mismatch') if $cursor != $content;
    } else {
        my ($cursor, $active, $start) = (0, undef, 0);
        for my $run (@{$runs}) {
            my $end = $cursor + $run->{modules};
            if ($run->{role} ne 'inter-character-gap') {
                my $source_index = $run->{sourceIndex};
                $source_index += 0;
                my $key = join "\x1f", $run->{sourceLabel}, $source_index, $run->{role};
                if (!defined($active) || $key ne $active->{key}) {
                    push @layouts, {
                        label => $active->{label}, startModule => $start,
                        endModule => $cursor, sourceIndex => $active->{sourceIndex},
                        role => $active->{role},
                    } if defined $active;
                    $active = { key => $key, label => $run->{sourceLabel},
                        sourceIndex => 0 + $run->{sourceIndex}, role => $run->{role} };
                    $start = $cursor;
                }
            }
            $cursor = $end;
        }
        push @layouts, {
            label => $active->{label}, startModule => $start, endModule => $cursor,
            sourceIndex => $active->{sourceIndex}, role => $active->{role},
        } if defined $active;
        _v1_fail('too-many-symbols') if @layouts > $MAX_SYMBOLS;
    }
    return {
        leftQuietZoneModules => 0 + $quiet, rightQuietZoneModules => 0 + $quiet,
        contentModules => $content, totalModules => $quiet + $content + $quiet,
        symbolLayouts => \@layouts,
    };
}

sub _v1_metadata {
    my ($metadata) = @_;
    $metadata //= {};
    _v1_fail('metadata-too-large') if keys(%{$metadata}) > $MAX_METADATA_ENTRIES;
    my ($total, %copy) = (0);
    require Encode;
    for my $key (keys %{$metadata}) {
        my $value = $metadata->{$key};
        _v1_fail('metadata-too-large') if ref($value);
        _v1_fail('metadata-too-large')
            if _v1_scalars($key, 'metadata-too-large') > $MAX_METADATA_KEY_SCALARS
                || _v1_scalars($value, 'metadata-too-large') > $MAX_METADATA_VALUE_SCALARS;
        $total += length(Encode::encode('UTF-8', $key)) + length(Encode::encode('UTF-8', $value));
        _v1_fail('metadata-too-large') if $total > $MAX_METADATA_UTF8_BYTES;
        $copy{$key} = "$value";
    }
    return \%copy;
}

sub project_barcode_1d_scene_v1 {
    my ($class, $runs, $quiet, $options) = @_;
    $options //= {};
    my $config = $options->{render_config} // {};
    _v1_fail('human-readable-text-unsupported')
        if $config->{include_human_readable_text} || defined $options->{human_readable_text};
    my $module_width = $config->{module_width} // 4;
    my $bar_height = $config->{bar_height} // 120;
    my $foreground = $config->{foreground} // '#000000';
    my $background = $config->{background} // '#ffffff';
    _v1_fail('invalid-render-config')
        unless _v1_integer($module_width) && $module_width >= 1 && $module_width <= $MAX_MODULE_WIDTH
            && _v1_integer($bar_height) && $bar_height >= 1 && $bar_height <= $MAX_BAR_HEIGHT
            && _v1_scalars($foreground, 'invalid-render-config') <= $MAX_COLOR_SCALARS
            && _v1_scalars($background, 'invalid-render-config') <= $MAX_COLOR_SCALARS;
    my $layout = $class->compute_barcode_1d_layout_v1($runs, $quiet, $options->{symbols});
    my $metadata = _v1_metadata($options->{metadata});
    my $label = $options->{label} // '1D barcode';
    _v1_fail('metadata-too-large')
        if _v1_scalars($label, 'metadata-too-large') > $MAX_LABEL_SCALARS;
    my ($cursor, @instructions) = ($quiet);
    for my $run (@{$runs}) {
        my $end = $cursor + $run->{modules};
        if ($run->{color} eq 'bar') {
            my $source_index = $run->{sourceIndex};
            $source_index += 0;
            push @instructions, CodingAdventures::PaintInstructions->paint_rect(
                $cursor * $module_width, 0, $run->{modules} * $module_width,
                $bar_height, $foreground,
                { sourceLabel => $run->{sourceLabel}, sourceIndex => "$source_index",
                  role => $run->{role}, moduleStart => "$cursor", moduleEnd => "$end" },
            );
        }
        $cursor = $end;
    }
    my $scene_width = $layout->{totalModules} * $module_width;
    my %canonical = (
        label => $label, leftQuietZoneModules => "$layout->{leftQuietZoneModules}",
        rightQuietZoneModules => "$layout->{rightQuietZoneModules}",
        contentModules => "$layout->{contentModules}", totalModules => "$layout->{totalModules}",
        moduleWidthPx => "$module_width", barHeightPx => "$bar_height",
        sceneWidthPx => "$scene_width", sceneHeightPx => "$bar_height",
        symbolCount => "@{[scalar @{$layout->{symbolLayouts}}]}",
    );
    return CodingAdventures::PaintInstructions->paint_scene(
        $scene_width, $bar_height, \@instructions, $background,
        { %{$metadata}, %canonical },
    );
}

1;
