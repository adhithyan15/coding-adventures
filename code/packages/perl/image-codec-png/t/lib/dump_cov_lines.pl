use strict;
use warnings;
use B;

use lib 'lib';
use lib '../pixel-container/lib';
use lib '../zip/lib';
use lib '../lzss/lib';

require CodingAdventures::ImageCodecPNG;
require CodingAdventures::ImageCodecPNG::Constants;
require CodingAdventures::ImageCodecPNG::Messages;
require CodingAdventures::ImageCodecPNG::Error;
require CodingAdventures::ImageCodecPNG::Adler32;
require CodingAdventures::ImageCodecPNG::Chunk;
require CodingAdventures::ImageCodecPNG::Filter;
require CodingAdventures::ImageCodecPNG::Decoder;
require CodingAdventures::ImageCodecPNG::Encoder;
require CodingAdventures::ImageCodecPNG::Codec;

my $out_file = $ARGV[0] or die "Usage: dump_cov_lines.pl <output_file>\n";

my @modules = qw(
    lib/CodingAdventures/ImageCodecPNG.pm
    lib/CodingAdventures/ImageCodecPNG/Constants.pm
    lib/CodingAdventures/ImageCodecPNG/Messages.pm
    lib/CodingAdventures/ImageCodecPNG/Error.pm
    lib/CodingAdventures/ImageCodecPNG/Adler32.pm
    lib/CodingAdventures/ImageCodecPNG/Chunk.pm
    lib/CodingAdventures/ImageCodecPNG/Filter.pm
    lib/CodingAdventures/ImageCodecPNG/Decoder.pm
    lib/CodingAdventures/ImageCodecPNG/Encoder.pm
    lib/CodingAdventures/ImageCodecPNG/Codec.pm
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
