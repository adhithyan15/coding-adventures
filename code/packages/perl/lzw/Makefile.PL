use ExtUtils::MakeMaker;
WriteMakefile(
    NAME             => 'CodingAdventures::LZW',
    VERSION_FROM     => 'lib/CodingAdventures/LZW.pm',
    ABSTRACT         => 'LZW lossless compression algorithm (1984) from scratch',
    AUTHOR           => 'Adhithya Rajasekaran',
    MIN_PERL_VERSION => '5.016',
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
);
