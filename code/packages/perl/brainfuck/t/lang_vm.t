use strict;
use warnings;
use Test2::V0;
use CodingAdventures::Brainfuck::LangVm;

my $result = CodingAdventures::Brainfuck::LangVm::execute_on_lang_vm('+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++.');
is($result->{output}, 'A', 'LANG VM executes output');

my $loop = CodingAdventures::Brainfuck::LangVm::execute_on_lang_vm('+++[>+++++++++++++++++++++<-]>++.');
is($loop->{output}, 'A', 'LANG VM executes loops');

done_testing();
