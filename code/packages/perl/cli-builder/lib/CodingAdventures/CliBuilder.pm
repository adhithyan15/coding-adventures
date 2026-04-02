package CodingAdventures::CliBuilder;

# ============================================================================
# CodingAdventures::CliBuilder — Declarative CLI Argument Parser
# ============================================================================
#
# # The Core Insight: CLI Syntax Is a Directed Graph
# ====================================================
#
# A CLI tool's valid syntax forms a directed graph:
#
#   git remote add <name> <url>
#
# The user navigates: root → remote → add → (two positional args).
# Valid invocations = valid paths through the graph from root to a leaf.
#
# Flag constraints (conflicts, requirements) form a second graph.
# Cycles in that graph represent spec bugs caught at load time.
#
# # Three-Phase Parsing
# ======================
#
# Phase 1 — ROUTING:
#   Walk argv, consuming tokens that match subcommand names.
#
# Phase 2 — SCANNING:
#   Classify remaining tokens:
#     "--"             → end_of_flags
#     "--name"         → long flag (value from next token)
#     "--name=value"   → long flag with inline value
#     "-x"             → short flag
#     "-xVALUE"        → short flag with inline value (non-boolean)
#     "-xyz"           → stacked boolean flags
#     other            → positional
#
# Phase 3 — VALIDATION:
#   Resolve positionals → argument slots, validate flag constraints.
#
# # Return Values
# ================
#
#   { type => 'result',  flags => {}, arguments => {}, command_path => [] }
#   { type => 'help',    text  => '...', command_path => [] }
#   { type => 'version', version => '1.0.0' }
#   { type => 'error',   errors => [{error_type=>, message=>}, ...] }

use strict;
use warnings;
use Carp qw(croak);

our $VERSION = '0.01';

# ============================================================================
# TokenClassifier
# ============================================================================

package CodingAdventures::CliBuilder::TokenClassifier;

use strict;
use warnings;

our $VERSION = '0.01';

sub _build_maps {
  my ($flags) = @_;
  my (%short, %long, %sdl);
  for my $f (@$flags) {
    $short{$f->{short}}            = $f if $f->{short};
    $long{$f->{long}}              = $f if $f->{long};
    $sdl{$f->{single_dash_long}}   = $f if $f->{single_dash_long};
  }
  return (\%short, \%long, \%sdl);
}

sub classify {
  my ($class, $token, $flags) = @_;
  my ($short, $long, $sdl) = _build_maps($flags);

  # "--" → end_of_flags
  return { kind => 'end_of_flags' } if $token eq '--';

  # Long flags: "--name" or "--name=value"
  if ($token =~ /^--(.+)$/) {
    my $rest = $1;
    if ($rest =~ /^([^=]+)=(.*)$/) {
      return { kind => 'long_flag_value', name => $1, value => $2 };
    }
    return { kind => 'long_flag', name => $rest };
  }

  # Bare "-" → positional
  return { kind => 'positional', value => '-' } if $token eq '-';

  # Short flags
  if ($token =~ /^-(.+)$/) {
    my $rest = $1;

    # Rule 1: single-dash-long exact match
    return { kind => 'single_dash_long', name => $rest } if exists $sdl->{$rest};

    # Rule 2: first char is a known short flag
    my $first = substr($rest, 0, 1);
    if (exists $short->{$first}) {
      my $f         = $short->{$first};
      my $remainder = substr($rest, 1);

      if ($remainder eq '') {
        return { kind => 'short_flag', char => $first };
      } elsif ($f->{type} eq 'boolean' || $f->{type} eq 'count') {
        # Try to stack remaining chars
        my @chars = ($first);
        my $ok    = 1;
        for my $ch (split //, $remainder) {
          if (exists $short->{$ch}) {
            push @chars, $ch;
          } else {
            $ok = 0; last;
          }
        }
        return $ok
          ? { kind => 'stacked_flags', chars => \@chars }
          : { kind => 'unknown_flag',  token => $token };
      } else {
        return { kind => 'short_flag_value', char => $first, value => $remainder };
      }
    }

    return { kind => 'unknown_flag', token => $token };
  }

  return { kind => 'positional', value => $token };
}

# ============================================================================
# SpecLoader
# ============================================================================

package CodingAdventures::CliBuilder::SpecLoader;

use strict;
use warnings;
use Carp qw(croak);

our $VERSION = '0.01';

my %VALID_TYPES = map { $_ => 1 }
  qw(boolean string integer float path file directory enum count);

sub _normalise_flag {
  my ($f, $scope) = @_;
  croak "$scope: flag missing 'id'"                   unless $f->{id};
  croak "$scope: flag '$f->{id}' missing 'description'" unless $f->{description};
  croak "$scope: flag '$f->{id}' missing 'type'"        unless $f->{type};
  croak "$scope: flag '$f->{id}' has invalid type '$f->{type}'"
    unless $VALID_TYPES{$f->{type}};
  if ($f->{type} eq 'enum') {
    croak "$scope: flag '$f->{id}' type=enum requires non-empty enum_values"
      unless ref($f->{enum_values}) eq 'ARRAY' && @{$f->{enum_values}};
  }
  return {
    id               => $f->{id},
    short            => $f->{short},
    long             => $f->{long},
    single_dash_long => $f->{single_dash_long},
    description      => $f->{description},
    type             => $f->{type},
    required         => $f->{required}  ? 1 : 0,
    default          => $f->{default},
    value_name       => $f->{value_name} // 'VALUE',
    enum_values      => $f->{enum_values} // [],
    conflicts_with   => $f->{conflicts_with} // [],
    requires         => $f->{requires}  // [],
    required_unless  => $f->{required_unless} // [],
    repeatable       => $f->{repeatable} ? 1 : 0,
  };
}

sub _normalise_argument {
  my ($a, $scope) = @_;
  croak "$scope: argument missing 'id'"                   unless $a->{id};
  croak "$scope: argument '$a->{id}' missing 'description'" unless $a->{description};
  return {
    id           => $a->{id},
    display_name => $a->{display_name} // uc($a->{id}),
    description  => $a->{description},
    required     => defined($a->{required}) ? $a->{required} : 1,
    variadic     => $a->{variadic} ? 1 : 0,
    type         => $a->{type} // 'string',
  };
}

sub _normalise_commands {
  my ($cmds, $scope) = @_;
  return [] unless $cmds;
  my @result;
  for my $cmd (@$cmds) {
    croak "$scope: command missing 'name'"        unless $cmd->{name};
    croak "$scope: command missing 'description'" unless $cmd->{description};
    push @result, {
      name        => $cmd->{name},
      description => $cmd->{description},
      flags       => [ map { _normalise_flag($_, $cmd->{name}) }
                       @{$cmd->{flags} // []} ],
      arguments   => [ map { _normalise_argument($_, $cmd->{name}) }
                       @{$cmd->{arguments} // []} ],
      commands    => _normalise_commands($cmd->{commands}, $cmd->{name}),
    };
  }
  return \@result;
}

sub load_hashref {
  my ($class, $raw) = @_;
  croak "spec must be a hash-ref" unless ref $raw eq 'HASH';

  croak "unsupported cli_builder_spec_version: " . ($raw->{cli_builder_spec_version} // 'undef')
    unless ($raw->{cli_builder_spec_version} // '') eq '1.0';
  croak "spec missing 'name'"        unless $raw->{name};
  croak "spec missing 'description'" unless $raw->{description};

  my $bf = $raw->{builtin_flags} // {};
  return {
    name          => $raw->{name},
    display_name  => $raw->{display_name} // $raw->{name},
    description   => $raw->{description},
    version       => $raw->{version},
    parsing_mode  => $raw->{parsing_mode} // 'gnu',
    builtin_flags => {
      help    => (exists $bf->{help})    ? $bf->{help}    : 1,
      version => (exists $bf->{version}) ? $bf->{version} : 1,
    },
    global_flags  => [ map { _normalise_flag($_, 'global') }
                       @{$raw->{global_flags} // []} ],
    flags         => [ map { _normalise_flag($_, 'root') }
                       @{$raw->{flags} // []} ],
    arguments     => [ map { _normalise_argument($_, 'root') }
                       @{$raw->{arguments} // []} ],
    commands      => _normalise_commands($raw->{commands}, 'root'),
  };
}

# ============================================================================
# HelpGenerator
# ============================================================================

package CodingAdventures::CliBuilder::HelpGenerator;

use strict;
use warnings;

our $VERSION = '0.01';

sub _flag_usage {
  my ($f) = @_;
  my @parts;
  push @parts, "-$f->{short}" if $f->{short};
  if ($f->{long}) {
    if ($f->{type} eq 'boolean' || $f->{type} eq 'count') {
      push @parts, "--$f->{long}";
    } else {
      push @parts, "--$f->{long} <$f->{value_name}>";
    }
  }
  return join(', ', @parts);
}

sub generate {
  my ($class, $spec, $command_path) = @_;

  # Navigate to target command node
  my $node = $spec;
  for my $i (1..$#$command_path) {
    my $name  = $command_path->[$i];
    my $found = (grep { $_->{name} eq $name } @{$node->{commands} // []})[0];
    $node = $found if $found;
  }

  my $name = join(' ', @$command_path);
  my @lines;

  # Usage
  my @usage = ($name);
  my @all_flags = (@{$spec->{global_flags}}, @{$node->{flags} // $spec->{flags} // []});
  push @usage, '[OPTIONS]' if @all_flags;
  push @usage, '[COMMAND]' if @{$node->{commands} // []};
  for my $a (@{$node->{arguments} // $spec->{arguments} // []}) {
    my ($open, $close) = $a->{required} ? ('<', '>') : ('[', ']');
    my $dots = $a->{variadic} ? '...' : '';
    push @usage, "${open}$a->{display_name}${dots}${close}";
  }

  push @lines, "USAGE\n  " . join(' ', @usage), '';

  # Description
  my $desc = $node->{description} // $spec->{description};
  push @lines, "DESCRIPTION\n  $desc", '';

  # Commands
  my @cmds = @{$node->{commands} // []};
  if (@cmds) {
    push @lines, 'COMMANDS';
    for my $cmd (@cmds) {
      push @lines, sprintf("  %-20s%s", $cmd->{name}, $cmd->{description});
    }
    push @lines, '';
  }

  # Options
  my @node_flags = @{$node->{flags} // $spec->{flags} // []};
  if (@node_flags) {
    push @lines, 'OPTIONS';
    for my $f (@node_flags) {
      my $left  = _flag_usage($f);
      my $right = $f->{description};
      $right .= " [default: $f->{default}]" if defined $f->{default};
      push @lines, sprintf("  %-30s%s", $left, $right);
    }
    push @lines, '';
  }

  # Global options + builtins
  my @global   = @{$spec->{global_flags} // []};
  my @builtins;
  push @builtins, { short=>'h', long=>'help', type=>'boolean',
    description=>'Show this help message and exit.' }
    if $spec->{builtin_flags}{help};
  push @builtins, { short=>undef, long=>'version', type=>'boolean',
    description=>'Show version and exit.' }
    if $spec->{builtin_flags}{version} && $spec->{version};
  my @all_global = (@global, @builtins);
  if (@all_global) {
    push @lines, 'GLOBAL OPTIONS';
    for my $f (@all_global) {
      push @lines, sprintf("  %-30s%s", _flag_usage($f), $f->{description});
    }
  }

  return join("\n", @lines);
}

# ============================================================================
# FlagValidator
# ============================================================================

package CodingAdventures::CliBuilder::FlagValidator;

use strict;
use warnings;

our $VERSION = '0.01';

sub validate {
  my ($class, $flags, $flag_defs) = @_;
  my @errors;

  for my $f (@$flag_defs) {
    my $id  = $f->{id};
    my $val = $flags->{$id};

    # Required check
    if ($f->{required} && (!defined $val || $val eq '' || $val eq '0')) {
      if (@{$f->{required_unless} // []}) {
        my $has = grep { defined $flags->{$_} && $flags->{$_} && $flags->{$_} ne '0' }
                  @{$f->{required_unless}};
        push @errors, "required flag missing: --" . ($f->{long} // $id)
          unless $has;
      } else {
        push @errors, "required flag missing: --" . ($f->{long} // $id);
      }
    }

    # Enum validation
    if ($f->{type} eq 'enum' && defined $val) {
      my $valid = grep { $_ eq $val } @{$f->{enum_values}};
      push @errors, sprintf("invalid value '%s' for --%s: must be one of: %s",
        $val, $f->{long} // $id, join(', ', @{$f->{enum_values}}))
        unless $valid;
    }

    # Conflicts
    if (defined $val && $val && $val ne '0') {
      for my $other_id (@{$f->{conflicts_with} // []}) {
        my $ov = $flags->{$other_id};
        if (defined $ov && $ov && $ov ne '0') {
          push @errors, sprintf("--%s conflicts with --%s",
            $f->{long} // $id, $other_id);
        }
      }
      for my $req_id (@{$f->{requires} // []}) {
        my $rv = $flags->{$req_id};
        unless (defined $rv && $rv && $rv ne '0') {
          push @errors, sprintf("--%s requires --%s", $f->{long} // $id, $req_id);
        }
      }
    }
  }

  return @errors;
}

# ============================================================================
# Parser
# ============================================================================

package CodingAdventures::CliBuilder::Parser;

use strict;
use warnings;
use Scalar::Util qw(looks_like_number);

our $VERSION = '0.01';

sub _coerce {
  my ($raw, $type, $id) = @_;
  if ($type eq 'boolean') {
    return (defined $raw && $raw eq 'true') ? 1 : 1;
  } elsif ($type eq 'integer' || $type eq 'count') {
    die "flag --$id expects an integer, got '$raw'\n"
      unless looks_like_number($raw);
    return int($raw);
  } elsif ($type eq 'float') {
    die "flag --$id expects a number, got '$raw'\n"
      unless looks_like_number($raw);
    return $raw + 0;
  } else {
    return "$raw";
  }
}

sub parse {
  my ($class, $spec, $argv) = @_;
  my @errors;

  # ── Phase 1: Routing ────────────────────────────────────────────────────
  my @command_path  = ($spec->{name});
  my $current_node  = $spec;
  my @remaining;
  my $routed        = 0;

  for my $i (0..$#$argv) {
    my $token = $argv->[$i];
    if (!$routed) {
      if ($token =~ /^-/) {
        # Flag tokens are never subcommand names — pass to scanner but keep routing
        push @remaining, $token;
      } else {
        my ($match) = grep { $_->{name} eq $token } @{$current_node->{commands} // []};
        if ($match) {
          push @command_path, $match->{name};
          $current_node = $match;
        } else {
          $routed = 1;
          push @remaining, @{$argv}[$i..$#$argv];
          last;
        }
      }
    }
  }

  # ── Build flag lookups ──────────────────────────────────────────────────
  my @all_flag_defs;
  push @all_flag_defs, @{$spec->{global_flags} // []};
  push @all_flag_defs, @{$current_node->{flags} // $spec->{flags} // []};

  # Builtins
  push @all_flag_defs, {
    id=>'__help__', short=>'h', long=>'help', type=>'boolean',
    description=>'Show help', required=>0, default=>0,
    value_name=>'', enum_values=>[], conflicts_with=>[],
    requires=>[], required_unless=>[], repeatable=>0, single_dash_long=>undef,
  } if $spec->{builtin_flags}{help};

  push @all_flag_defs, {
    id=>'__version__', short=>undef, long=>'version', type=>'boolean',
    description=>'Show version', required=>0, default=>0,
    value_name=>'', enum_values=>[], conflicts_with=>[],
    requires=>[], required_unless=>[], repeatable=>0, single_dash_long=>undef,
  } if $spec->{builtin_flags}{version} && $spec->{version};

  my (%by_short, %by_long, %by_sdl);
  for my $f (@all_flag_defs) {
    $by_short{$f->{short}}            = $f if $f->{short};
    $by_long{$f->{long}}              = $f if $f->{long};
    $by_sdl{$f->{single_dash_long}}   = $f if $f->{single_dash_long};
  }

  # ── Phase 2: Scanning ───────────────────────────────────────────────────
  my (%flags, @positionals, @explicit_flags);
  my $end_of_flags       = 0;
  my $expecting_value_for;

  my $set_flag = sub {
    my ($f, $value) = @_;
    my $id = $f->{id};
    if ($f->{type} eq 'count') {
      $flags{$id} = ($flags{$id} // 0) + 1;
    } elsif ($f->{repeatable}) {
      $flags{$id} //= [];
      push @{$flags{$id}}, _coerce($value, $f->{type}, $id);
    } else {
      $flags{$id} = ($f->{type} eq 'boolean') ? 1 : _coerce($value, $f->{type}, $id);
    }
    push @explicit_flags, $id;
  };

  for my $token (@remaining) {
    if ($expecting_value_for) {
      my $f = $expecting_value_for;
      $expecting_value_for = undef;
      $set_flag->($f, $token);
      next;
    }

    if ($end_of_flags) {
      push @positionals, $token;
      next;
    }

    my $cls = CodingAdventures::CliBuilder::TokenClassifier->classify($token, \@all_flag_defs);

    if ($cls->{kind} eq 'end_of_flags') {
      $end_of_flags = 1;

    } elsif ($cls->{kind} eq 'long_flag') {
      my $f = $by_long{$cls->{name}};
      unless ($f) {
        push @errors, { error_type => 'unknown_flag', message => "unknown flag: --$cls->{name}" };
        next;
      }
      if ($f->{type} eq 'boolean') { $set_flag->($f, 1) }
      elsif ($f->{type} eq 'count')   { $set_flag->($f, 1) }
      else { $expecting_value_for = $f }
      return { type => 'help', text => CodingAdventures::CliBuilder::HelpGenerator->generate($spec, \@command_path), command_path => \@command_path }
        if $f->{id} eq '__help__';
      return { type => 'version', version => $spec->{version} }
        if $f->{id} eq '__version__';

    } elsif ($cls->{kind} eq 'long_flag_value') {
      my $f = $by_long{$cls->{name}};
      unless ($f) {
        push @errors, { error_type => 'unknown_flag', message => "unknown flag: --$cls->{name}" };
        next;
      }
      $set_flag->($f, $cls->{value});

    } elsif ($cls->{kind} eq 'short_flag') {
      my $f = $by_short{$cls->{char}};
      unless ($f) {
        push @errors, { error_type => 'unknown_flag', message => "unknown flag: -$cls->{char}" };
        next;
      }
      if ($f->{type} eq 'boolean') { $set_flag->($f, 1) }
      elsif ($f->{type} eq 'count')   { $set_flag->($f, 1) }
      else { $expecting_value_for = $f }
      return { type => 'help', text => CodingAdventures::CliBuilder::HelpGenerator->generate($spec, \@command_path), command_path => \@command_path }
        if $f->{id} eq '__help__';
      return { type => 'version', version => $spec->{version} }
        if $f->{id} eq '__version__';

    } elsif ($cls->{kind} eq 'short_flag_value') {
      my $f = $by_short{$cls->{char}};
      unless ($f) {
        push @errors, { error_type => 'unknown_flag', message => "unknown flag: -$cls->{char}" };
        next;
      }
      $set_flag->($f, $cls->{value});

    } elsif ($cls->{kind} eq 'stacked_flags') {
      for my $ch (@{$cls->{chars}}) {
        my $f = $by_short{$ch};
        unless ($f) {
          push @errors, { error_type => 'unknown_flag', message => "unknown flag in stack: -$ch" };
          next;
        }
        $set_flag->($f, 1);
        return { type => 'help', text => CodingAdventures::CliBuilder::HelpGenerator->generate($spec, \@command_path), command_path => \@command_path }
          if $f->{id} eq '__help__';
        return { type => 'version', version => $spec->{version} }
          if $f->{id} eq '__version__';
      }

    } elsif ($cls->{kind} eq 'single_dash_long') {
      my $f = $by_sdl{$cls->{name}};
      unless ($f) {
        push @errors, { error_type => 'unknown_flag', message => "unknown flag: -$cls->{name}" };
        next;
      }
      if ($f->{type} eq 'boolean') { $set_flag->($f, 1) }
      else { $expecting_value_for = $f }

    } elsif ($cls->{kind} eq 'unknown_flag') {
      push @errors, { error_type => 'unknown_flag', message => "unknown flag: $cls->{token}" };

    } elsif ($cls->{kind} eq 'positional') {
      push @positionals, $cls->{value};
    }
  }

  if ($expecting_value_for) {
    push @errors, { error_type => 'missing_flag_value',
      message => "flag --" . ($expecting_value_for->{long} // $expecting_value_for->{id}) . " requires a value" };
  }

  # ── Phase 3: Validation ──────────────────────────────────────────────────
  # Apply defaults
  for my $f (@all_flag_defs) {
    unless (exists $flags{$f->{id}}) {
      if ($f->{type} eq 'boolean') {
        $flags{$f->{id}} = $f->{default} // 0;
      } elsif ($f->{type} eq 'count') {
        $flags{$f->{id}} = $f->{default} // 0;
      } else {
        $flags{$f->{id}} = $f->{default};
      }
    }
  }

  # Resolve positionals
  my %arguments;
  my @arg_defs = @{$current_node->{arguments} // $spec->{arguments} // []};
  my $pos_idx  = 0;

  for my $a (@arg_defs) {
    if ($a->{variadic}) {
      my @vals = @positionals[$pos_idx..$#positionals];
      $pos_idx  = @positionals;
      if ($a->{required} && !@vals) {
        push @errors, { error_type => 'missing_argument',
          message => "required argument <$a->{display_name}> is missing" };
      }
      $arguments{$a->{id}} = \@vals;
    } else {
      if ($pos_idx <= $#positionals) {
        $arguments{$a->{id}} = $positionals[$pos_idx++];
      } elsif ($a->{required}) {
        push @errors, { error_type => 'missing_argument',
          message => "required argument <$a->{display_name}> is missing" };
      }
    }
  }

  # Flag validation
  my @flag_errors = CodingAdventures::CliBuilder::FlagValidator->validate(\%flags, \@all_flag_defs);
  push @errors, map { { error_type => 'flag_error', message => $_ } } @flag_errors;

  # Remove builtins
  delete @flags{qw(__help__ __version__)};

  return { type => 'error', errors => \@errors } if @errors;

  return {
    type           => 'result',
    program        => $spec->{name},
    command_path   => \@command_path,
    flags          => \%flags,
    arguments      => \%arguments,
    explicit_flags => \@explicit_flags,
  };
}

# ============================================================================
# Public API (main package)
# ============================================================================

package CodingAdventures::CliBuilder;

sub parse_hashref {
  my ($class, $spec_raw, $argv) = @_;
  my $spec = CodingAdventures::CliBuilder::SpecLoader->load_hashref($spec_raw);
  return CodingAdventures::CliBuilder::Parser->parse($spec, $argv);
}

1;
__END__

=head1 NAME

CodingAdventures::CliBuilder - Declarative CLI argument parser

=head1 SYNOPSIS

  use CodingAdventures::CliBuilder;

  my $spec = {
    cli_builder_spec_version => '1.0',
    name        => 'myapp',
    description => 'My application',
    flags => [{
      id => 'verbose', short => 'v', long => 'verbose',
      description => 'Verbose output', type => 'boolean',
    }],
    arguments => [{
      id => 'file', display_name => 'FILE',
      description => 'Input file', required => 1,
    }],
    commands => [],
  };

  my $result = CodingAdventures::CliBuilder->parse_hashref($spec, ['--verbose', 'input.txt']);
  if ($result->{type} eq 'result') {
    print $result->{flags}{verbose};     # 1
    print $result->{arguments}{file};    # input.txt
  }

=cut
