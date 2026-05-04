use ExtUtils::MakeMaker;
WriteMakefile(
    NAME => 'CodingAdventures::NeuralNetwork',
    VERSION_FROM => 'lib/CodingAdventures/NeuralNetwork.pm',
    PREREQ_PM => {},
    TEST_REQUIRES => { 'Test::More' => 0 },
);
