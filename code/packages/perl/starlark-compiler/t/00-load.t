use strict;
use warnings;
use v5.28;
use Test2::V0;

ok(eval { require CodingAdventures::StarlarkCompiler; 1 }, 'CodingAdventures::StarlarkCompiler loads');
ok(eval { require CodingAdventures::StarlarkCompiler::Constants; 1 }, 'Constants loads');
ok(eval { require CodingAdventures::StarlarkCompiler::Messages; 1 }, 'Messages loads');
ok(eval { require CodingAdventures::StarlarkCompiler::Errors; 1 }, 'Errors loads');
ok(eval { require CodingAdventures::StarlarkCompiler::Opcodes; 1 }, 'Opcodes loads');

done_testing;
