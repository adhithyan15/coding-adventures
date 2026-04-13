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

1;
