#!/usr/bin/env perl

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;

use CodingAdventures::BuildTool::CIWorkflow;

subtest 'allows toolchain-scoped dotnet changes' => sub {
    my $change = CodingAdventures::BuildTool::CIWorkflow::analyze_patch(<<'PATCH');
@@ -312,0 +313,6 @@
+      - name: Set up .NET
+        if: needs.detect.outputs.needs_dotnet == 'true'
+        uses: actions/setup-dotnet@v4
+        with:
+          dotnet-version: '9.0.x'
PATCH

    ok(!$change->{requires_full_rebuild}, 'dotnet-only workflow change stays scoped');
    is(
        [ CodingAdventures::BuildTool::CIWorkflow::sorted_toolchains($change->{toolchains}) ],
        ['dotnet'],
        'detects the dotnet toolchain',
    );
};

subtest 'ignores comment-only changes' => sub {
    my $change = CodingAdventures::BuildTool::CIWorkflow::analyze_patch(<<'PATCH');
@@ -316,2 +316,2 @@
-          # .NET 8 is the current LTS release.
+          # .NET 9 is the current LTS release.
PATCH

    ok(!$change->{requires_full_rebuild}, 'comment-only change stays scoped');
    is(
        [ CodingAdventures::BuildTool::CIWorkflow::sorted_toolchains($change->{toolchains}) ],
        [],
        'comment-only change does not claim toolchains',
    );
};

subtest 'requires a full rebuild for build command changes' => sub {
    my $change = CodingAdventures::BuildTool::CIWorkflow::analyze_patch(<<'PATCH');
@@ -404,1 +404,1 @@
-          $BT -root . -validate-build-files -language all
+          $BT -root . -force -validate-build-files -language all
PATCH

    ok($change->{requires_full_rebuild}, 'shared build command changes remain unsafe');
};

subtest 'requires a full rebuild for unknown workflow changes' => sub {
    my $change = CodingAdventures::BuildTool::CIWorkflow::analyze_patch(<<'PATCH');
@@ -170,0 +171,1 @@
+      timeout-minutes: 45
PATCH

    ok($change->{requires_full_rebuild}, 'unknown workflow edits stay conservative');
};

done_testing();
