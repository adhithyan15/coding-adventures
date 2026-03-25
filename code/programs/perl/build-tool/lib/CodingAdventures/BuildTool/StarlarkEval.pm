package CodingAdventures::BuildTool::StarlarkEval;

# StarlarkEval.pm -- Starlark BUILD File Detection and Rule Mapping
# =================================================================
#
# Starlark is a dialect of Python used as a build description language by
# Bazel, Buck, and similar tools. Some BUILD files in this monorepo use
# Starlark rules instead of raw shell commands:
#
#   load("//tools/rules:perl.bzl", "perl_library")
#
#   perl_library(
#     name = "logic-gates",
#     srcs = glob(["lib/**/*.pm"]),
#     deps = ["//code/packages/perl/arithmetic:arithmetic"],
#   )
#
# This module:
#   1. Detects whether a BUILD file uses Starlark syntax.
#   2. Extracts rule names, targets, and arguments from the BUILD file.
#   3. Maps rule names to shell commands that implement the rule's semantics.
#
# Detection heuristics:
#   - A `load(...)` call at the top of the file → definitely Starlark.
#   - A known rule name followed by `(` → likely Starlark.
#   - Pure shell commands (no function calls) → not Starlark.
#
# Rule-to-command mapping:
#   perl_library / perl_binary  → cpanm --installdeps --quiet . && prove -l -v t/
#   py_library / py_binary      → uv pip install -e ".[dev]" && pytest
#   go_library / go_binary      → go build && go test && go vet
#   ruby_library                → bundle install && rake test
#   ts_library / ts_binary      → npm install && npx vitest run
#   rust_library / rust_binary  → cargo build && cargo test
#   elixir_library              → mix deps.get && mix test
#   lua_library                 → luarocks make && busted
#
# Perl advantages demonstrated here:
#   - Regex for Starlark detection — one-liners replace multi-pass parsers.
#   - Dispatch tables (hashes of anonymous subs) instead of if-elsif chains.
#   - Named captures (?<name>...) for readable regex groups.

use strict;
use warnings;

our $VERSION = '0.01';

# KNOWN_RULES -- set of Starlark rule names we recognise.
#
# If any of these appears followed by '(' in a BUILD file, the file is
# treated as Starlark. We use a hash for O(1) lookup.
my %KNOWN_RULES = map { $_ => 1 } qw(
    perl_library perl_binary
    py_library py_binary
    go_library go_binary
    ruby_library ruby_binary
    ts_library ts_binary
    rust_library rust_binary
    elixir_library elixir_binary
    lua_library lua_binary
);

# RULE_COMMANDS -- maps rule name to the list of shell commands it generates.
my %RULE_COMMANDS = (
    perl_library    => ["cpanm --installdeps --quiet .", "prove -l -v t/"],
    perl_binary     => ["cpanm --installdeps --quiet .", "prove -l -v t/"],

    py_library      => ['uv pip install --system -e ".[dev]"',
                        "python -m pytest --cov --cov-report=term-missing"],
    py_binary       => ['uv pip install --system -e ".[dev]"',
                        "python -m pytest --cov --cov-report=term-missing"],

    go_library      => ["go build ./...", "go test ./... -v -cover", "go vet ./..."],
    go_binary       => ["go build ./...", "go test ./... -v -cover", "go vet ./..."],

    ruby_library    => ["bundle install --quiet", "bundle exec rake test"],
    ruby_binary     => ["bundle install --quiet", "bundle exec rake test"],

    ts_library      => ["npm install --silent", "npx vitest run"],
    ts_binary       => ["npm install --silent", "npx vitest run"],

    rust_library    => ["cargo build", "cargo test"],
    rust_binary     => ["cargo build", "cargo test"],

    elixir_library  => ["mix deps.get --quiet", "mix test"],
    elixir_binary   => ["mix deps.get --quiet", "mix test"],

    lua_library     => ["luarocks make", "busted"],
    lua_binary      => ["luarocks make", "busted"],
);

# new -- Constructor.
sub new {
    my ($class) = @_;
    return bless {}, $class;
}

# is_starlark -- Detect whether a BUILD file uses Starlark syntax.
#
# @param $content -- Contents of the BUILD file as a string.
# @return 1 if Starlark, 0 if plain shell.
sub is_starlark {
    my ($self, $content) = @_;
    return 0 unless defined $content && $content ne '';

    # A load() call is definitive.
    return 1 if $content =~ /^\s*load\s*\(/m;

    # Any known rule name followed by '(' on a non-comment line.
    # We strip comment lines first, then check for rule patterns.
    (my $no_comments = $content) =~ s/^\s*#.*$//mg;
    for my $rule (keys %KNOWN_RULES) {
        return 1 if $no_comments =~ /\b\Q$rule\E\s*\(/;
    }

    return 0;
}

# extract_targets -- Parse Starlark targets from a BUILD file.
#
# Returns a list of target hashrefs:
#   {
#     rule => "perl_library",
#     name => "logic-gates",
#     srcs => ["lib/**/*.pm"],
#     deps => ["//code/packages/perl/arithmetic:arithmetic"],
#   }
#
# Our parser handles the common subset of Starlark:
#   - Rule calls: rule_name(key = value, ...)
#   - String values: "..." or '...'
#   - List values: [item, ...]
#   - No nested function calls (we don't evaluate glob())
#
# This is not a full Starlark parser. It handles the patterns that appear
# in this monorepo's BUILD files.
sub extract_targets {
    my ($self, $content) = @_;
    my @targets;

    # Match top-level rule calls: RULE_NAME( ... ) across multiple lines.
    # We use a simple bracket-counting approach to find the matching ')'.
    my @lines = split /\n/, $content;
    my $i = 0;
    while ($i < @lines) {
        my $line = $lines[$i];

        # Skip comments.
        if ($line =~ /^\s*#/) {
            $i++;
            next;
        }

        # Look for a known rule name starting a call.
        if ($line =~ /^\s*(\w+)\s*\(/) {
            my $rule = $1;
            if (exists $KNOWN_RULES{$rule}) {
                # Accumulate lines until we have a balanced call.
                my $body = '';
                my $depth = 0;
                my $j = $i;
                while ($j < @lines) {
                    $body .= $lines[$j] . "\n";
                    $depth += () = $lines[$j] =~ /\(/g;
                    $depth -= () = $lines[$j] =~ /\)/g;
                    $j++;
                    last if $depth <= 0;
                }

                my $target = $self->_parse_rule_call($rule, $body);
                push @targets, $target if $target;
                $i = $j;
                next;
            }
        }
        $i++;
    }

    return @targets;
}

# _parse_rule_call -- Extract name, srcs, deps from a rule call body.
#
# @param $rule -- Rule name string.
# @param $body -- The full text of the rule call.
# @return target hashref or undef.
sub _parse_rule_call {
    my ($self, $rule, $body) = @_;

    # Extract name = "value".
    my ($name) = ($body =~ /name\s*=\s*["']([^"']+)["']/);
    return undef unless $name;

    # Extract srcs = ["...", "..."] or srcs = glob(["...", "..."]).
    my @srcs;
    if ($body =~ /srcs\s*=\s*(?:glob\s*\()?\s*\[([^\]]*)\]/s) {
        my $srcs_text = $1;
        @srcs = ($srcs_text =~ /["']([^"']+)["']/g);
    }

    # Extract deps = ["...", "..."].
    my @deps;
    if ($body =~ /deps\s*=\s*\[([^\]]*)\]/s) {
        my $deps_text = $1;
        @deps = ($deps_text =~ /["']([^"']+)["']/g);
    }

    return {
        rule => $rule,
        name => $name,
        srcs => \@srcs,
        deps => \@deps,
    };
}

# generate_commands -- Return the shell commands for a given rule name.
#
# @param $rule -- Rule name string (e.g. "perl_library").
# @return list of shell command strings, or empty list if unknown rule.
sub generate_commands {
    my ($self, $rule) = @_;
    return @{ $RULE_COMMANDS{$rule} // [] };
}

# commands_for_build_file -- Parse a BUILD file and return its commands.
#
# If the file is Starlark, generates commands from the rule mapping.
# If the file is plain shell, returns the non-blank, non-comment lines.
#
# @param $content -- BUILD file contents.
# @return list of command strings.
sub commands_for_build_file {
    my ($self, $content) = @_;

    unless ($self->is_starlark($content)) {
        # Plain shell: return non-blank, non-comment lines.
        return grep { $_ ne '' && !/^#/ }
               map  { s/^\s+|\s+$//gr }
               split /\n/, $content;
    }

    # Starlark: generate commands from rules.
    my @targets = $self->extract_targets($content);
    my @cmds;
    for my $t (@targets) {
        push @cmds, $self->generate_commands($t->{rule});
    }
    return @cmds;
}

1;

__END__

=head1 NAME

CodingAdventures::BuildTool::StarlarkEval - Starlark BUILD file detection and mapping

=head1 SYNOPSIS

  use CodingAdventures::BuildTool::StarlarkEval;

  my $se = CodingAdventures::BuildTool::StarlarkEval->new();

  if ($se->is_starlark($build_content)) {
      my @targets = $se->extract_targets($build_content);
      for my $t (@targets) {
          my @cmds = $se->generate_commands($t->{rule});
      }
  }

=head1 DESCRIPTION

Detects whether a BUILD file uses Starlark syntax and maps rule names to
shell commands. Supports perl_library, py_library, go_library, ruby_library,
ts_library, rust_library, elixir_library, and lua_library.

=cut
