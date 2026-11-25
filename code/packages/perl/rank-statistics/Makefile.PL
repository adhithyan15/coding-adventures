use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::RankStatistics',
    VERSION_FROM     => 'lib/CodingAdventures/RankStatistics.pm',
    ABSTRACT         => 'Sample medians and exact distribution-free median intervals',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'Math::BigInt' => 0,
        'POSIX'        => 0,
    },
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
    META_MERGE       => { 'meta-spec' => { version => 2 }, resources => { repository => { type => 'git', url => 'https://github.com/adhithyan15/coding-adventures.git' } } },
);
