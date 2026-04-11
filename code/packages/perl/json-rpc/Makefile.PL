use ExtUtils::MakeMaker;
WriteMakefile(
    NAME         => 'CodingAdventures::JsonRpc',
    VERSION_FROM => 'lib/CodingAdventures/JsonRpc.pm',
    TEST_REQUIRES => { 'Test2::V0' => 0 },
    META_MERGE => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/example/coding-adventures',
            },
        },
    },
);
