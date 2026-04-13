package CodingAdventures::PaintCodecPngNative;

use strict;
use warnings;
use DynaLoader;

our $VERSION = '0.01';
our @ISA = ('DynaLoader');

sub dl_load_flags { 0x01 }

__PACKAGE__->bootstrap($VERSION);

1;
