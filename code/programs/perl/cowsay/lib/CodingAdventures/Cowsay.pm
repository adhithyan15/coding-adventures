package CodingAdventures::Cowsay;

# cowsay — routed through paint-vm-ascii (Perl port)
#
# Third language in the cowsay-through-paint-vm-ascii rollout (after csharp
# and fsharp). Everything up through composing the bubble+cow text block is
# ordinary string formatting, ported unchanged from the reference
# implementation at code/programs/go/cowsay/main.go. The one thing that's
# different from that reference: instead of printing the composed text
# directly, build_scene converts it into a PaintScene of glyph_run
# instructions (one glyph placement per non-space character, positioned on
# an 8x16 character grid), and CodingAdventures::PaintVmAscii->render turns
# that scene back into the terminal string we print. This is also the PR
# that brought perl/paint-vm-ascii up from a rect-only stub to the full
# P2D02-paint-vm-ascii.md contract (see that package's CHANGELOG).

use strict;
use warnings;
use utf8;

use File::Spec;
use CodingAdventures::PaintInstructions;
use CodingAdventures::PaintVmAscii;

our $VERSION = '0.1.0';

# paint-vm-ascii's documented default scale factors (P2D02-paint-vm-ascii.md).
use constant {
    SCALE_X => 8,
    SCALE_Y => 16,
};

my %MODE_OVERRIDES = (
    borg     => { eyes => '==', tongue => undef },
    dead     => { eyes => 'XX', tongue => 'U ' },
    greedy   => { eyes => '$$', tongue => undef },
    paranoid => { eyes => '@@', tongue => undef },
    stoned   => { eyes => 'xx', tongue => 'U ' },
    tired    => { eyes => '--', tongue => undef },
    wired    => { eyes => 'OO', tongue => undef },
    youthful => { eyes => '..', tongue => undef },
);

our @MODE_FLAG_IDS = qw(borg dead greedy paranoid stoned tired wired youthful);

# ---------------------------------------------------------------------------
# Rendering core (ported from code/programs/go/cowsay/main.go)
# ---------------------------------------------------------------------------

# Splits text into lines no longer than $width, breaking on word boundaries.
# A single word longer than the width is kept whole (never split mid-word).
sub wrap_text {
    my ($text, $width) = @_;
    return [$text] if length($text) <= $width;

    my @words = grep { length($_) > 0 } split(/ /, $text);
    return [''] unless @words;

    my @lines;
    my $current = '';
    for my $word (@words) {
        if (length($current) + length($word) + 1 <= $width) {
            $current = length($current) == 0 ? $word : "$current $word";
        }
        else {
            push @lines, $current if length($current) > 0;
            $current = $word;
        }
    }
    push @lines, $current if length($current) > 0;

    return \@lines;
}

# Draws the speech/thought bubble around the given lines. A single line gets
# "< ... >" (or "( ... )" for a thought bubble); multiple lines get
# "/ ... \", "| ... |", "\ ... /" (or "( ... )" on every line for a thought
# bubble).
sub format_bubble {
    my ($lines, $is_think) = @_;
    return '' unless @$lines;

    my $max_len = 0;
    for my $line (@$lines) {
        $max_len = length($line) if length($line) > $max_len;
    }

    my $border_top    = ' ' . ('_' x ($max_len + 2));
    my $border_bottom = ' ' . ('-' x ($max_len + 2));

    my @body;
    if (@$lines == 1) {
        my ($start, $end) = $is_think ? ('(', ')') : ('<', '>');
        push @body, sprintf('%s %-*s %s', $start, $max_len, $lines->[0], $end);
    }
    else {
        my $count = scalar @$lines;
        for my $i (0 .. $count - 1) {
            my ($start, $end);
            if ($is_think) {
                ($start, $end) = ('(', ')');
            }
            elsif ($i == 0) {
                ($start, $end) = ('/', '\\');
            }
            elsif ($i == $count - 1) {
                ($start, $end) = ('\\', '/');
            }
            else {
                ($start, $end) = ('|', '|');
            }
            push @body, sprintf('%s %-*s %s', $start, $max_len, $lines->[$i], $end);
        }
    }

    return join("\n", $border_top, @body, $border_bottom);
}

# Pads or truncates a mode string (eyes/tongue) to exactly two characters,
# matching cowsay's convention that eyes/tongue are always a 2-char glyph.
sub normalize_two_chars {
    my ($value) = @_;
    return substr($value . '  ', 0, 2) if length($value) < 2;
    return substr($value, 0, 2)        if length($value) > 2;
    return $value;
}

# Applies mode shortcuts (--borg, --dead, etc.) on top of the base
# eyes/tongue flag values, then normalizes both to two characters. Modes are
# mutually exclusive per cowsay.json, but this accepts any set for
# robustness.
sub resolve_eyes_and_tongue {
    my ($base_eyes, $base_tongue, $active_modes) = @_;
    my $eyes   = $base_eyes;
    my $tongue = $base_tongue;

    for my $mode (@$active_modes) {
        my $override = $MODE_OVERRIDES{$mode} or next;
        $eyes = $override->{eyes};
        $tongue = $override->{tongue} if defined $override->{tongue};
    }

    return (normalize_two_chars($eyes), normalize_two_chars($tongue));
}

# Loads a .cow template's body from $cows_dir, falling back to default.cow
# when the requested file doesn't exist. The template is a Perl heredoc
# ($the_cow = <<EOC; ... EOC); only the body between the heredoc markers is
# returned.
#
# $cow_name comes from the user-supplied -f/--file flag, so it is treated as
# untrusted: only a bare filename (no directory separators, no
# rooted/absolute path) is accepted, and the resolved path is verified to
# stay inside $cows_dir before it's read -- otherwise this falls back to
# default.cow instead of reading an arbitrary file the caller pointed at via
# "..", a rooted override, or similar (mirrors the fix applied to the C# and
# F# pilots' load_cow after /security-review).
sub load_cow {
    my ($cow_name, $cows_dir) = @_;
    my $cows_root = File::Spec->rel2abs($cows_dir);

    my ($volume, $directories, $safe_name) = File::Spec->splitpath($cow_name);
    my $is_rooted = File::Spec->file_name_is_absolute($cow_name);

    my $cow_path;
    if (length($safe_name) > 0 && !$is_rooted) {
        $cow_path = File::Spec->rel2abs(File::Spec->catfile($cows_root, "$safe_name.cow"));
    }

    # Defense-in-depth: confirm the resolved path is genuinely inside
    # $cows_root by checking that its path relative to $cows_root never
    # climbs via "..". Structurally this can't happen given $safe_name is
    # guaranteed separator-free above, but this doesn't rely on that
    # invariant holding forever.
    my $is_within_cows_dir = 0;
    if (defined $cow_path) {
        my $relative = File::Spec->abs2rel($cow_path, $cows_root);
        my @segments = File::Spec->splitdir($relative);
        $is_within_cows_dir = !grep { $_ eq File::Spec->updir } @segments;
    }

    if (!defined($cow_path) || !$is_within_cows_dir || !-e $cow_path) {
        $cow_path = File::Spec->catfile($cows_root, 'default.cow');
    }

    open(my $fh, '<:encoding(UTF-8)', $cow_path) or die "cowsay: cannot read $cow_path: $!\n";
    local $/;
    my $content = <$fh>;
    close $fh;

    if ($content =~ /<<EOC;\n(.*?)EOC/s) {
        return $1;
    }
    return $content;
}

# Walks up from $start_dir looking for CLAUDE.md, the repo-root sentinel
# file. CLAUDE.md (not code/specs/cowsay.json itself) is used deliberately —
# it's a more robust marker than reaching for the very file being located,
# and this exact fix was called out as a lesson from a prior, reverted
# cowsay Lua port's CI pathing problems (PR #1535).
sub find_repo_root {
    my ($start_dir) = @_;
    my $dir = $start_dir;

    for (1 .. 24) {
        return $dir if -e File::Spec->catfile($dir, 'CLAUDE.md');
        my $parent = File::Spec->catdir($dir, File::Spec->updir);
        my $resolved = File::Spec->rel2abs($parent);
        last if $resolved eq File::Spec->rel2abs($dir);
        $dir = $resolved;
    }

    return $start_dir;
}

# Composes the full bubble+cow text block for one invocation — everything up
# to (but not including) the paint-vm-ascii render step. $invocation is a
# hashref with keys: message, eyes, tongue, active_modes (arrayref), nowrap,
# width, think, cowfile.
sub compose_content {
    my ($invocation, $cows_dir) = @_;

    my ($eyes, $tongue) = resolve_eyes_and_tongue(
        $invocation->{eyes}, $invocation->{tongue}, $invocation->{active_modes},
    );

    my @lines;
    for my $raw_line (split(/\n/, $invocation->{message}, -1)) {
        if (length($raw_line) == 0) {
            push @lines, '';
        }
        elsif ($invocation->{nowrap}) {
            push @lines, $raw_line;
        }
        else {
            push @lines, @{ wrap_text($raw_line, $invocation->{width}) };
        }
    }

    my $thoughts = $invocation->{think} ? 'o' : '\\';
    my $bubble = format_bubble(\@lines, $invocation->{think});

    my $cow_template = load_cow($invocation->{cowfile}, $cows_dir);

    my $cow = $cow_template;
    $cow =~ s/\$eyes/$eyes/g;
    $cow =~ s/\$tongue/$tongue/g;
    $cow =~ s/\$thoughts/$thoughts/g;
    $cow =~ s/\\\\/\\/g;

    return "$bubble\n$cow";
}

# Converts a composed text block into a PaintScene: one glyph_run
# instruction per line, one glyph placement per non-space character. See
# code/specs/cowsay-paintvm-pipeline.md §3 for the full contract, including
# why glyph_id is a literal Unicode code point here (an ASCII-backend-only
# relaxation of the general PaintGlyphRun contract).
sub build_scene {
    my ($text) = @_;
    (my $normalized = $text) =~ s/\r\n/\n/g;
    my @lines = split(/\n/, $normalized, -1);

    my $max_width = 0;
    my @instructions;

    for my $row (0 .. $#lines) {
        my $line = $lines[$row];
        $max_width = length($line) if length($line) > $max_width;

        my @glyphs;
        my @chars = split(//, $line);
        for my $col (0 .. $#chars) {
            my $ch = $chars[$col];
            next if $ch eq ' ';
            push @glyphs, {
                glyph_id => ord($ch),
                x        => $col * SCALE_X,
                y        => $row * SCALE_Y,
            };
        }

        if (@glyphs) {
            push @instructions, {
                kind      => 'glyph_run',
                glyphs    => \@glyphs,
                font_ref  => 'terminal-mono',
                font_size => SCALE_Y,
                fill      => '#000000',
            };
        }
    }

    my $width  = (($max_width > 0 ? $max_width : 1)) * SCALE_X;
    my $height = ((@lines > 0 ? scalar(@lines) : 1)) * SCALE_Y;

    return CodingAdventures::PaintInstructions->paint_scene($width, $height, \@instructions, 'transparent');
}

# End-to-end: compose the bubble+cow text, build a PaintScene from it, and
# render that scene through paint-vm-ascii.
sub render {
    my ($invocation, $cows_dir) = @_;
    my $content = compose_content($invocation, $cows_dir);
    my $scene   = build_scene($content);
    return CodingAdventures::PaintVmAscii->render($scene, { scale_x => SCALE_X, scale_y => SCALE_Y });
}

# ---------------------------------------------------------------------------
# CLI glue — the bridge between CliBuilder's flags/arguments hashes and the
# typed invocation this module renders. Kept in this module (rather than
# cowsay.pl) so it's directly unit-testable without spawning a process or
# driving a real Parser.
# ---------------------------------------------------------------------------

sub is_list_requested {
    my ($flags) = @_;
    return $flags->{list} ? 1 : 0;
}

# Cow file basenames under $cows_dir, sorted ordinally (ASCIIbetical, Perl's
# default string `cmp`, matches every other port's StringComparer.Ordinal).
sub list_cow_files {
    my ($cows_dir) = @_;
    opendir(my $dh, $cows_dir) or die "cowsay: cannot read $cows_dir: $!\n";
    my @names = sort grep { s/\.cow$// } grep { /\.cow$/ } readdir($dh);
    closedir $dh;
    return \@names;
}

# Resolves the message from the parsed "message" positional argument.
# Returns undef when no message was given on argv — the caller should fall
# back to stdin.
sub resolve_message_from_arguments {
    my ($arguments) = @_;
    my $message_arg = $arguments->{message};
    return undef unless ref($message_arg) eq 'ARRAY' && @$message_arg;
    return join(' ', map { defined($_) ? $_ : '' } @$message_arg);
}

# Builds an invocation hashref from a resolved message and the parsed flags
# hashref, applying cowsay.json's documented defaults for any flag that
# wasn't explicitly set.
sub build_invocation {
    my ($message, $flags) = @_;

    my $width = 40;
    if (defined $flags->{width}) {
        $width = $flags->{width};
        $width = 1 if $width < 1;
        $width = 2_147_483_647 if $width > 2_147_483_647;    # clamp to a 32-bit int ceiling, like every other port
        $width = int($width);
    }

    my @active_modes = grep { $flags->{$_} } @MODE_FLAG_IDS;

    return {
        message      => $message,
        eyes         => defined($flags->{eyes})    ? $flags->{eyes}    : 'oo',
        tongue       => defined($flags->{tongue})  ? $flags->{tongue}  : '  ',
        active_modes => \@active_modes,
        nowrap       => $flags->{nowrap} ? 1 : 0,
        width        => $width,
        think        => $flags->{think} ? 1 : 0,
        cowfile      => defined($flags->{cowfile}) ? $flags->{cowfile} : 'default',
    };
}

1;
