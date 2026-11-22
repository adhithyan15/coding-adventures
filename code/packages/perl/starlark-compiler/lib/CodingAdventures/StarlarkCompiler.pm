package CodingAdventures::StarlarkCompiler;

use strict;
use warnings;
use v5.28;

our $VERSION = '0.1.0';

use parent 'Exporter';

use CodingAdventures::StarlarkCompiler::Constants;
use CodingAdventures::StarlarkCompiler::Messages;
use CodingAdventures::StarlarkCompiler::Errors;
use CodingAdventures::StarlarkCompiler::Opcodes;

our @EXPORT_OK = (
    @CodingAdventures::StarlarkCompiler::Constants::EXPORT,
    @CodingAdventures::StarlarkCompiler::Messages::EXPORT,
    @CodingAdventures::StarlarkCompiler::Errors::EXPORT,
    @CodingAdventures::StarlarkCompiler::Opcodes::EXPORT,
);

our %EXPORT_TAGS = (
    all => \@EXPORT_OK,
);

1;
