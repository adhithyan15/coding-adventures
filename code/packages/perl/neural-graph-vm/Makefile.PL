use ExtUtils::MakeMaker;
WriteMakefile(
    NAME => 'CodingAdventures::NeuralGraphVM',
    VERSION_FROM => 'lib/CodingAdventures/NeuralGraphVM.pm',
    PREREQ_PM => {},
    TEST_REQUIRES => { 'Test::More' => 0 },
);
