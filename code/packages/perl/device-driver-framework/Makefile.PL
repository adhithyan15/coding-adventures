use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::DeviceDriverFramework',
    VERSION_FROM     => 'lib/CodingAdventures/DeviceDriverFramework.pm',
    ABSTRACT_FROM    => 'lib/CodingAdventures/DeviceDriverFramework.pm',
    AUTHOR           => 'coding-adventures',
    MIN_PERL_VERSION => '5.020',
    TEST_REQUIRES    => {
        'Test2::V0' => '0',
    },
    META_MERGE => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/coding-adventures/coding-adventures.git',
            },
        },
    },
);
