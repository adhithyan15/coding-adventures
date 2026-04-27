use ExtUtils::MakeMaker;
WriteMakefile(
    NAME         => 'CodingAdventures::Ls00',
    VERSION_FROM => 'lib/CodingAdventures/Ls00.pm',
    PREREQ_PM    => {},
    TEST_REQUIRES => { 'Test::More' => 0 },
);
