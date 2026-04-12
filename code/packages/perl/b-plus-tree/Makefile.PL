use ExtUtils::MakeMaker;
WriteMakefile(
    NAME             => 'CodingAdventures::BPlusTree',
    VERSION_FROM     => 'lib/CodingAdventures/BPlusTree.pm',
    ABSTRACT_FROM    => 'lib/CodingAdventures/BPlusTree.pm',
    MIN_PERL_VERSION => '5.016',
    TEST_REQUIRES    => { 'Test2::V0' => '0' },
    META_MERGE       => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/coding-adventures/coding-adventures.git',
            },
        },
    },
);
