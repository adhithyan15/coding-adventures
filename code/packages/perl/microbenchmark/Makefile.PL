use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::Microbenchmark',
    VERSION_FROM     => 'lib/CodingAdventures/Microbenchmark.pm',
    ABSTRACT         => 'Paired benchmark collection, exact rank analysis, and report replay',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    EXE_FILES        => ['script/array-microbenchmark'],
    PREREQ_PM        => {
        'CodingAdventures::RankStatistics' => '0.1.0',
        'CodingAdventures::JsonParser'     => '0.01',
        'JSON::PP'                         => 0,
        'Math::BigInt'                     => 0,
        'Math::BigFloat'                   => 0,
        'Time::HiRes'                      => 0,
    },
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
    META_MERGE       => { 'meta-spec' => { version => 2 }, resources => { repository => { type => 'git', url => 'https://github.com/adhithyan15/coding-adventures.git' } } },
);
