#!/usr/bin/env perl
# cowsay (Perl) — entry point
#
# Thin CLI wiring: parse argv against code/specs/cowsay.json via CliBuilder,
# resolve the parsed flags/arguments into an invocation hashref, and hand
# off to CodingAdventures::Cowsay::render for the actual formatting +
# paint-vm-ascii render. See code/specs/cowsay-paintvm-pipeline.md for the
# design.
#
# Unlike the C#/F# ports, CodingAdventures::CliBuilder->parse_hashref takes
# argv WITHOUT a leading program-name placeholder (its Parser iterates the
# whole array from index 0) -- @ARGV is passed straight through.

use strict;
use warnings;
use utf8;
binmode(STDOUT, ':encoding(UTF-8)');
binmode(STDERR, ':encoding(UTF-8)');

use FindBin qw($Bin);
use lib "$Bin/lib";
use lib "$Bin/../../../packages/perl/cli-builder/lib";
use lib "$Bin/../../../packages/perl/paint-instructions/lib";
use lib "$Bin/../../../packages/perl/paint-vm-ascii/lib";

use Cwd qw(getcwd);
use File::Spec;
use JSON::PP qw(decode_json);
use CodingAdventures::CliBuilder;
use CodingAdventures::Cowsay;

my $repo_root = CodingAdventures::Cowsay::find_repo_root(getcwd());
my $spec_path = File::Spec->catfile($repo_root, 'code', 'specs', 'cowsay.json');
my $cows_dir  = File::Spec->catfile($repo_root, 'code', 'specs', 'cows');

open(my $spec_fh, '<:encoding(UTF-8)', $spec_path)
    or die "cowsay: cannot read spec file '$spec_path': $!\n";
my $spec_json = do { local $/; <$spec_fh> };
close $spec_fh;

my $spec_raw = decode_json($spec_json);
my $result   = CodingAdventures::CliBuilder->parse_hashref($spec_raw, [@ARGV]);

if ($result->{type} eq 'help') {
    print $result->{text}, "\n";
    exit 0;
}
elsif ($result->{type} eq 'version') {
    print $result->{version}, "\n";
    exit 0;
}
elsif ($result->{type} eq 'error') {
    print STDERR join("\n", map { $_->{message} } @{ $result->{errors} }), "\n";
    exit 1;
}

my $flags     = $result->{flags};
my $arguments = $result->{arguments};

if (CodingAdventures::Cowsay::is_list_requested($flags)) {
    print "$_\n" for @{ CodingAdventures::Cowsay::list_cow_files($cows_dir) };
    exit 0;
}

my $message = CodingAdventures::Cowsay::resolve_message_from_arguments($arguments);
if (!defined $message) {
    if (-t STDIN) {    ## no critic (InputOutput::ProhibitInteractiveTest)
        exit 0;
    }
    $message = do { local $/; <STDIN> };
    $message = '' unless defined $message;
    $message =~ s/^\s+|\s+$//g;
}

exit 0 if length($message) == 0;

my $invocation = CodingAdventures::Cowsay::build_invocation($message, $flags);
print CodingAdventures::Cowsay::render($invocation, $cows_dir), "\n";
