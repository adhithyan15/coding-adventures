use ExtUtils::MakeMaker;
WriteMakefile(
  NAME             => 'CodingAdventures::CorrelationVector',
  VERSION_FROM     => 'lib/CodingAdventures/CorrelationVector.pm',
  ABSTRACT         => 'Correlation Vector — append-only provenance tracking for any entity',
  PREREQ_PM        => {},
  TEST_REQUIRES    => { 'Test2::V0' => 0 },
  META_MERGE       => {
    'meta-spec' => { version => 2 },
    resources   => { homepage => 'https://github.com/adhithyan15/coding-adventures' },
  },
);
