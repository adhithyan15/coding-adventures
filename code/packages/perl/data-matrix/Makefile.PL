use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::DataMatrix',
    AUTHOR           => 'Adhithya Rajasekaran <adhithyan15@gmail.com>',
    VERSION_FROM     => 'lib/CodingAdventures/DataMatrix.pm',
    ABSTRACT         => 'ISO/IEC 16022:2006 Data Matrix ECC200 encoder',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.020',
    PREREQ_PM        => {
        'CodingAdventures::Barcode2D' => '0.1.0',
        'Carp'                        => 0,
        'POSIX'                       => 0,
    },
    TEST_REQUIRES    => {
        'Test::More' => 0,
    },
    META_MERGE => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/adhithyan/coding-adventures.git',
                web  => 'https://github.com/adhithyan/coding-adventures',
            },
        },
    },
);
