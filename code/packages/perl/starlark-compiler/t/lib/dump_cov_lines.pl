use strict;
use warnings;
use B;

use lib 'lib';
require CodingAdventures::StarlarkCompiler;
require CodingAdventures::StarlarkCompiler::Constants;
require CodingAdventures::StarlarkCompiler::Messages;
require CodingAdventures::StarlarkCompiler::Errors;
require CodingAdventures::StarlarkCompiler::Opcodes;

my $out_file = $ARGV[0] or die "Usage: dump_cov_lines.pl <output_file>\n";

my @modules = qw(
    lib/CodingAdventures/StarlarkCompiler.pm
    lib/CodingAdventures/StarlarkCompiler/Constants.pm
    lib/CodingAdventures/StarlarkCompiler/Messages.pm
    lib/CodingAdventures/StarlarkCompiler/Errors.pm
    lib/CodingAdventures/StarlarkCompiler/Opcodes.pm
);

open my $fh, '>', $out_file or die "Cannot open $out_file: $!\n";

for my $mod (@modules) {
    # Search %main:: for matching debugger file symbol
    for my $k (keys %main::) {
        next unless $k =~ /^_</;
        my $clean_k = $k;
        $clean_k =~ s{^_<}{};
        $clean_k =~ tr{\\}{/};
        if ($clean_k =~ m{\Q$mod\E$}) {
            no strict 'refs';
            my @l = @{$k};
            my @res;
            for my $i (1 .. $#l) {
                if (defined $l[$i]) {
                    my $sv = B::svref_2object(\$l[$i]);
                    if ($sv && ($sv->FLAGS & B::SVf_IOK) && $sv->IV != 0) {
                        push @res, $i;
                    }
                }
            }
            print $fh "$mod\t" . join(',', @res) . "\n";
            last;
        }
    }
}

close $fh;
