package CodingAdventures::BrainfuckIrCompiler::BuildConfig;

# ============================================================================
# BuildConfig — controls what the brainfuck-ir-compiler emits
# ============================================================================
#
# Build modes are **composable flags**, not a fixed enum.  A BuildConfig
# object controls every aspect of compilation:
#
#   insert_bounds_checks — emit tape pointer range checks (debug builds)
#   insert_debug_locs    — emit source location COMMENT instructions
#   mask_byte_arithmetic — AND 0xFF after every cell mutation (correctness)
#   tape_size            — configurable tape length (default 30,000 cells)
#
# ## Presets
#
#   debug_config()   — bounds checks ON, debug locs ON, masking ON
#   release_config() — bounds checks OFF, debug locs OFF, masking ON
#
# ## Composability
#
# New modes can be added without modifying existing code — just construct a
# BuildConfig with the desired flags.  For example:
#
#   my $cfg = CodingAdventures::BrainfuckIrCompiler::BuildConfig->new(
#       insert_bounds_checks => 0,
#       insert_debug_locs    => 0,
#       mask_byte_arithmetic => 0,  # bare-metal, no wrapping
#       tape_size            => 1000,
#   );
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# new(%args) — construct a BuildConfig with explicit flag values.
#
# All fields have explicit defaults so callers can override just what they need.
sub new {
    my ($class, %args) = @_;
    return bless {
        insert_bounds_checks => $args{insert_bounds_checks} // 0,
        insert_debug_locs    => $args{insert_debug_locs}    // 0,
        mask_byte_arithmetic => $args{mask_byte_arithmetic} // 1,
        tape_size            => $args{tape_size}            // 30000,
    }, $class;
}

# debug_config() — preset suitable for debug builds.
#
# All safety checks are enabled.  Use this during development and testing.
sub debug_config {
    my ($class) = @_;
    return $class->new(
        insert_bounds_checks => 1,
        insert_debug_locs    => 1,
        mask_byte_arithmetic => 1,
        tape_size            => 30000,
    );
}

# release_config() — preset suitable for release builds.
#
# Safety checks are disabled for maximum performance.  Use this for
# production compilation of known-correct Brainfuck programs.
sub release_config {
    my ($class) = @_;
    return $class->new(
        insert_bounds_checks => 0,
        insert_debug_locs    => 0,
        mask_byte_arithmetic => 1,
        tape_size            => 30000,
    );
}

1;

__END__

=head1 NAME

CodingAdventures::BrainfuckIrCompiler::BuildConfig - compilation flags for the Brainfuck IR compiler

=head1 SYNOPSIS

  use CodingAdventures::BrainfuckIrCompiler::BuildConfig;

  my $cfg = CodingAdventures::BrainfuckIrCompiler::BuildConfig->debug_config;
  # $cfg->{insert_bounds_checks} == 1
  # $cfg->{insert_debug_locs}    == 1
  # $cfg->{mask_byte_arithmetic} == 1
  # $cfg->{tape_size}            == 30000

  my $rel = CodingAdventures::BrainfuckIrCompiler::BuildConfig->release_config;
  # $rel->{insert_bounds_checks} == 0

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
