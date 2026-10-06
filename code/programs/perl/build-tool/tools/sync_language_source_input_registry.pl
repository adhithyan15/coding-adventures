#!/usr/bin/env perl

# Copy the reviewed neutral registry into the Perl distribution. Runtime code
# reads only this installed package resource, never a checkout fixture.
use strict;
use warnings;
use FindBin qw($Bin);
use File::Copy qw(copy);
use File::Spec ();

my $source = File::Spec->catfile($Bin, qw(.. .. .. .. specs fixtures build-tool-v1 language-source-input-registry.json));
my $destination = File::Spec->catfile($Bin, qw(.. lib CodingAdventures BuildTool language-source-input-registry.json));
copy($source, $destination) or die "Cannot sync source-input registry: $!";
print "Synced language source-input registry\n";
