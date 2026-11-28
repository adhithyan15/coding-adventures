"""Exercise named DB05b and DB05d2b2a mutations in isolated local copies."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
GO_ROOT = PACKAGE.parent
CODE_ROOT = PACKAGE.parents[2]
FIXTURES = (
    "ftl-nand-relocation.json",
    "ftl-block-reclamation.json",
    "ftl-live-page-copy.json",
    "ftl-evacuation.json",
)

MUTATIONS = (
    (
        "ignore-caller-allocation-fence",
        "TestExternalFenceCombinesWithInternalFencesAndPreservesLiveReads",
        "external fence write selected",
        "relocation.go",
        "copy(fence, excludedPages)",
        "copy(fence, make([]bool, len(fence)))",
    ),
    (
        "publish-partial-destination",
        "TestPartialProgramIsNeverPublished",
        "partial destination must remain unpublished",
        "relocation.go",
        "disposition = ftlpagemap.AbortSpent",
        "disposition = ftlpagemap.MediaReady",
    ),
    (
        "reuse-spent-page",
        "TestSpentNANDPageIsNotSelectedAgain",
        "spent NAND page was selected again",
        "ftl-page-map/page_map.go",
        "if status == Free && (len(excluded) == 0 || !excluded[index]) {",
        "if status == Free || status == Stale {",
    ),
    (
        "acknowledge-media-before-host-completion",
        "TestAfterMediaPublicationDoesNotAcknowledge",
        "AfterMedia publishes without caller acknowledgement",
        "relocation.go",
        "receipt := Receipt{\n\t\tPhysical:     reservation.Physical,\n\t\tPublished:    published,\n\t\tAcknowledged: program.Acknowledged,\n\t}",
        "receipt := Receipt{\n\t\tPhysical:     reservation.Physical,\n\t\tPublished:    published,\n\t\tAcknowledged: published,\n\t}",
    ),
    (
        "alias-read-buffer",
        "TestReadsSnapshotsAndPayloadsAreDetached",
        "read output aliases device bytes",
        "relocation.go",
        "return append([]byte(nil), device.pages[int(physical)].Bytes...), true, nil",
        "return device.pages[int(physical)].Bytes, true, nil",
    ),
    (
        "release-before-full-media-result",
        "TestPartialEraseDoesNotReleaseQuarantinedExtent",
        "partial erase released quarantined extent",
        "relocation.go",
        "released := outcome.Block.CompletedErases > block.CompletedErases",
        "released := outcome.Block.CompletedErases >= block.CompletedErases",
    ),
    (
        "ignore-incomplete-block-allocation-fence",
        "TestWriteFenceSkipsEveryIncompleteBlock",
        "write allocation entered a quarantined erase block",
        "relocation.go",
        "if !block.IncompleteErase && (!device.evacuationActive || int64(blockIndex) != device.evacuationBlock) {",
        "if block.IncompleteErase || !device.evacuationActive || int64(blockIndex) != device.evacuationBlock {",
    ),
    (
        "lose-media-completion-release-evidence",
        "TestAfterMediaReleasesExtentWithoutAcknowledgement",
        "AfterMedia did not release the unacknowledged completed extent",
        "relocation.go",
        "released := outcome.Block.CompletedErases > block.CompletedErases",
        "released := outcome.Acknowledged",
    ),
    (
        "copy-wrong-source-bytes",
        "TestCopyUsesExactCurrentSourceBytes",
        "copy destination does not contain source page bytes",
        "relocation.go",
        "device.pages[int(move.SourcePhysical)].Bytes, cut",
        "device.pages[int(move.DestinationPhysical)].Bytes, cut",
    ),
    (
        "publish-partial-copy",
        "TestCopyPartialCutNeverPublishes",
        "partial copy changed logical mapping",
        "relocation.go",
        "case nandpageprogramcuts.AfterPrefix:\n\t\tdisposition = ftlpagemap.AbortSpent",
        "case nandpageprogramcuts.AfterPrefix:\n\t\tdisposition = ftlpagemap.MediaReady",
    ),
    (
        "reuse-spent-copy-destination",
        "TestSpentCopyDestinationCannotBeRetried",
        "spent destination retry did not return unavailable refusal",
        "relocation.go",
        "snapshot.PhysicalStatus[int(move.DestinationPhysical)] != ftlpagemap.Free",
        "false",
    ),
    (
        "allocate-inside-active-evacuation",
        "TestHostWriteCannotAllocateInsideActiveEvacuation",
        "active extent was not fenced",
        "relocation.go",
        "if !block.IncompleteErase && (!device.evacuationActive || int64(blockIndex) != device.evacuationBlock) {",
        "if !block.IncompleteErase {",
    ),
    (
        "reuse-previous-evacuation-destination-after-prefix",
        "TestCopyNextReplansInsteadOfReusingSpentDestination",
        "copy reused prior spent destination",
        "evacuation.go",
        "move := plan.Copies[0]",
        "move := plan.Copies[0]\n\tif device.pages[4].Programmed { move.DestinationPhysical = 4 }",
    ),
    (
        "erase-live-evacuation-target",
        "TestEvacuationProgressAndCopyReplanning",
        "live target must precede erase-cut validation",
        "evacuation.go",
        "if progress.RemainingLivePages != 0 {",
        "if false && progress.RemainingLivePages != 0 {",
    ),
    (
        "retain-fence-after-media-complete-lost-ack",
        "TestEvacuationPartialEraseLocksUntilMediaRelease",
        "unacknowledged completed erase did not clear the active target",
        "evacuation.go",
        "if receipt.Released {",
        "if receipt.Acknowledged {",
    ),
    (
        "ignore-retirement-incomplete-destination-fence",
        "TestRetirementDrainCombinesActiveAndIncompleteDestinationFences",
        "retirement plan =",
        "evacuation.go",
        "// Start from the device's detached quarantine mask, then add caller policy\n\t// and the active target. The source bit is allowed by PlanDrain.\n\tdestinationFences := device.incompleteBlockMask()",
        "destinationFences := make([]bool, len(device.blocks))",
    ),
    (
        "ignore-retirement-active-destination-fence",
        "TestRetirementDrainCombinesActiveAndIncompleteDestinationFences",
        "retirement plan =",
        "evacuation.go",
        "if device.evacuationActive {\n\t\tdestinationFences[int(device.evacuationBlock)] = true",
        "if false {\n\t\tdestinationFences[int(device.evacuationBlock)] = true",
    ),
    (
        "ignore-retirement-caller-mask",
        "TestRetirementDrainValidationOrderAndReadOnlyBudgetSemantics",
        "caller-fenced destination did not produce shortage",
        "evacuation.go",
        "for blockIndex, excluded := range excludedBlocks {",
        "for blockIndex, excluded := range make([]bool, len(device.blocks)) {",
    ),
    (
        "require-erase-budget-for-read-only-retirement-plan",
        "TestRetirementDrainValidationOrderAndReadOnlyBudgetSemantics",
        "zero-budget all-live source planning must be allowed",
        "evacuation.go",
        "if device.blocks[int(block)].IncompleteErase {\n\t\treturn ftlevacuationplan.Plan{}, ErrEvacuationBlockIncomplete\n\t}",
        "if device.blocks[int(block)].IncompleteErase {\n\t\treturn ftlevacuationplan.Plan{}, ErrEvacuationBlockIncomplete\n\t}\n\tif device.blocks[int(block)].CompletedErases >= device.maxCompletedErases {\n\t\treturn ftlevacuationplan.Plan{}, nandblockerasecuts.ErrEraseLimit\n\t}",
    ),
    (
        "accept-one-past-last-block",
        "TestLivePagesInBlockValidationOrderAndSignedBoundaries",
        "upper boundary refusal",
        "relocation.go",
        "block >= int64(len(device.blocks))",
        "block > int64(len(device.blocks))",
    ),
    (
        "count-non-live-map-slots",
        "TestLivePagesInBlockCountsOnlyCurrentMapOwners",
        "LivePagesInBlock(0) = 2, want 1",
        "relocation.go",
        "for physical := first; physical < end; physical++ {\n\t\tif snapshot.PhysicalStatus[int(physical)] == ftlpagemap.Live {\n\t\t\tlive++\n\t\t}",
        "for physical := first; physical < end; physical++ {\n\t\tif snapshot.PhysicalStatus[int(physical)] != ftlpagemap.Free {\n\t\t\tlive++\n\t\t}",
    ),
    (
        "owner-inventory-includes-next-block-boundary",
        "TestOwnerInventoryUsesCurrentLogicalMapAndAscendingLogicalOrder",
        "OwnersInBlock(0) = [{Logical:0 Physical:2} {Logical:1 Physical:0}]",
        "relocation.go",
        "physical >= first && physical < end",
        "physical >= first && physical <= end",
    ),
    (
        "owner-inventory-selects-stale-pages",
        "TestOwnerInventoryUsesCurrentLogicalMapAndAscendingLogicalOrder",
        "OwnersInBlock(0) = []",
        "relocation.go",
        "if physical >= first && physical < end && snapshot.PhysicalStatus[int(physical)] == ftlpagemap.Live {",
        "if physical >= first && physical < end && snapshot.PhysicalStatus[int(physical)] == ftlpagemap.Stale {",
    ),
    (
        "owner-inventory-duplicates-each-owner",
        "TestOwnerInventoryUsesCurrentLogicalMapAndAscendingLogicalOrder",
        "OwnersInBlock(0) = [{Logical:0 Physical:1} {Logical:0 Physical:1} {Logical:1 Physical:0} {Logical:1 Physical:0}]",
        "relocation.go",
        "owners = append(owners, PageOwner{Logical: int64(logical), Physical: physical})",
        "owners = append(owners, PageOwner{Logical: int64(logical), Physical: physical}, PageOwner{Logical: int64(logical), Physical: physical})",
    ),
    (
        "owner-inventory-reverses-logical-order",
        "TestOwnerInventoryUsesCurrentLogicalMapAndAscendingLogicalOrder",
        "OwnersInBlock(0) = [{Logical:1 Physical:0} {Logical:0 Physical:1}], want [{Logical:0 Physical:1} {Logical:1 Physical:0}]",
        "relocation.go",
        "owners = append(owners, PageOwner{Logical: int64(logical), Physical: physical})",
        "owners = append([]PageOwner{{Logical: int64(logical), Physical: physical}}, owners...)",
    ),
    (
        "lookup-validates-device-before-address",
        "TestLookupRejectsMalformedDeviceBeforeLogicalAddress",
        "zero-device Lookup(-1) = 0/false/invalid logical page; want geometry refusal",
        "relocation.go",
        "func (device Device) Lookup(logical int64) (physical int64, present bool, err error) {\n\tif !device.configured() {",
        "func (device Device) Lookup(logical int64) (physical int64, present bool, err error) {\n\tif logical < 0 { return 0, false, ftlpagemap.ErrInvalidLogicalPage };\n\tif !device.configured() {",
    ),
    (
        "lookup-preserves-absence",
        "TestLookupReturnsZeroForAbsence",
        "absent Lookup(0) = 0/true/<nil>; want zero/absent/nil",
        "relocation.go",
        "return device.table.Lookup(logical)",
        "physical, _, lookupErr := device.table.Lookup(logical)\n\tif lookupErr != nil { return 0, false, lookupErr }\n\treturn physical, true, nil",
    ),
    (
        "lookup-preserves-physical-zero-presence",
        "TestLookupMarksPhysicalPageZeroPresent",
        "mapped page-zero Lookup(0) = 0/false/<nil>; want zero/present/nil",
        "relocation.go",
        "return device.table.Lookup(logical)",
        "physical, present, lookupErr := device.table.Lookup(logical)\n\tif lookupErr != nil || !present { return physical, present, lookupErr }\n\tif physical == 0 { return physical, false, nil }\n\treturn physical, present, nil",
    ),
    (
        "lookup-preserves-read-bytes",
        "TestLookupPreservesReadBytesAndDetachedOwnership",
        "lookup-preserved read bytes = 0000/true/<nil>",
        "relocation.go",
        "return append([]byte(nil), device.pages[int(physical)].Bytes...), true, nil",
        "return make([]byte, len(device.pages[int(physical)].Bytes)), true, nil",
    ),
    (
        "refresh-owner-identity-before-media-publication",
        "TestOwnerPublicationIdentityChangesOnlyAtMediaPublication",
        "old identity match=false after cut",
        "relocation.go",
        "if published {\n\t\tnext.ownerIdentities = append([]*ownerPublicationIdentity(nil), device.ownerIdentities...)",
        "if true {\n\t\tnext.ownerIdentities = append([]*ownerPublicationIdentity(nil), device.ownerIdentities...)",
    ),
    (
        "reuse-owner-identity-across-sibling-publications",
        "TestSiblingPublicationsUseFreshIdentityAndPreserveCommonAncestors",
        "independent sibling publications shared a current-owner identity",
        "relocation.go",
        "next.ownerIdentities[int(reservation.Logical)] = &ownerPublicationIdentity{marker: 1}",
        "if device.ownerIdentities[int(reservation.Logical)] == nil {\n\t\t\tnext.ownerIdentities[int(reservation.Logical)] = &ownerPublicationIdentity{marker: 1}\n\t\t} else {\n\t\t\tnext.ownerIdentities[int(reservation.Logical)] = device.ownerIdentities[int(reservation.Logical)]\n\t\t}",
    ),
    (
        "ignore-owner-publication-identity",
        "TestSiblingPublicationsUseFreshIdentityAndPreserveCommonAncestors",
        "independent sibling publications shared a current-owner identity",
        "relocation.go",
        "return device.ownerIdentities[int(observation.Owner.Logical)] == observation.identity",
        "return true",
    ),
    (
        "ignore-owner-physical-coordinate",
        "TestOwnerPublicationIdentityChangesOnlyAtMediaPublication",
        "modified physical coordinate matched current owner",
        "relocation.go",
        "if err != nil || !present || physical != observation.Owner.Physical {",
        "if err != nil || !present || physical < 0 {",
    ),
    (
        "accept-stale-owner-on-invalidation",
        "TestInvalidateOwnerPreservesCutAndCopyPublicationRules",
        "old-observation error=current logical owner changed",
        "relocation.go",
        "if !device.MatchesOwner(observation) {",
        "if false {",
    ),
    (
        "retain-invalidated-publication-identity",
        "TestInvalidateOwnerRequiresCurrentOpaqueObservation",
        "success did not clear exactly the removed owner's identity",
        "relocation.go",
        "next.ownerIdentities[int(observation.Owner.Logical)] = nil",
        "next.ownerIdentities[int(observation.Owner.Logical)] = device.ownerIdentities[int(observation.Owner.Logical)]",
    ),
    (
        "invalidate-a-neighbor-publication",
        "TestInvalidateOwnerPreservesDetachedMediaAndOtherObservations",
        "owner invalidation=<nil>",
        "relocation.go",
        "next.ownerIdentities[int(observation.Owner.Logical)] = nil",
        "next.ownerIdentities = make([]*ownerPublicationIdentity, len(device.ownerIdentities))",
    ),
    (
        "observe-neighbor-block-coordinates",
        "TestObserveBlockPreservesPartiallyProgrammedPageBytes",
        "partially programmed page=",
        "relocation.go",
        "func (device Device) ObserveBlock(block int64) (nandblockerasecuts.Block, error) {\n\tsnapshot := device.table.Snapshot()\n\tfirst, end, err := device.validatedBlockRange(snapshot, block)",
        "func (device Device) ObserveBlock(block int64) (nandblockerasecuts.Block, error) {\n\tsnapshot := device.table.Snapshot()\n\tfirst, end, err := device.validatedBlockRange(snapshot, block+1)",
    ),
    (
        "alias-observed-page-bytes",
        "TestObserveBlockDeeplyDetachesPagesAndBytes",
        "mutating observation changed the device or its retained snapshot",
        "relocation.go",
        "observed.Pages[int(physical-first)] = nandblockerasecuts.Page{\n\t\t\tBytes:      append([]byte(nil), page.Bytes...),",
        "observed.Pages[int(physical-first)] = nandblockerasecuts.Page{\n\t\t\tBytes:      page.Bytes,",
    ),
    (
        "omit-program-marker-from-observation",
        "TestObserveBlockMatchesPhysicalSnapshotForEmptyLiveStaleAndActive",
        "ObserveBlock(0)=",
        "relocation.go",
        "Bytes:      append([]byte(nil), page.Bytes...),\n\t\t\tProgrammed: page.Programmed,",
        "Bytes:      append([]byte(nil), page.Bytes...),\n\t\t\tProgrammed: false,",
    ),
    (
        "omit-completed-erase-generation",
        "TestObserveBlockExposesEveryPartialEraseAndLostAckReuse",
        "ObserveBlock(0)=",
        "relocation.go",
        "CompletedErases: progress.CompletedErases,",
        "CompletedErases: 0,",
    ),
    (
        "unowned-program-fails-to-mark-page-stale",
        "TestProgramUnownedPageEveryCutAndPrefixMatchesIndependentModel",
        "mapping/page count mismatch",
        "relocation.go",
        "table, err = table.RecordUnownedProgram(physical)",
        "table, err = table.ReleaseErased(physical, 1)",
    ),
    (
        "unowned-program-receipt-uses-relative-page",
        "TestProgramUnownedPageEveryCutAndPrefixMatchesIndependentModel",
        "unowned program receipt=",
        "relocation.go",
        "Physical: physical, Programmed: program.State.Programmed,",
        "Physical: page, Programmed: program.State.Programmed,",
    ),
    (
        "unowned-after-media-is-acknowledged",
        "TestProgramUnownedPagePreservesOwnersViewsAndAcknowledgement",
        "AfterMedia receipt=",
        "relocation.go",
        "receipt := UnownedProgramReceipt{\n\t\tPhysical: physical, Programmed: program.State.Programmed,\n\t\tAcknowledged: program.Acknowledged,\n\t}",
        "receipt := UnownedProgramReceipt{\n\t\tPhysical: physical, Programmed: program.State.Programmed,\n\t\tAcknowledged: program.State.Programmed,\n\t}",
    ),
    (
        "omit-last-block-from-whole-media-capture",
        "TestObserveMediaMatchesEveryBlockThroughProgramAndEraseCuts",
        "ObserveMedia()=",
        "relocation.go",
        "for block := range device.blocks {\n\t\tfirst := int64(block) * device.pagesPerBlock",
        "for block := 0; block < len(device.blocks)-1; block++ {\n\t\tfirst := int64(block) * device.pagesPerBlock",
    ),
    (
        "alias-whole-media-page-bytes",
        "TestObserveMediaDeeplyDetachesEveryBlockAndCall",
        "mutating returned block or page changed device/next result",
        "relocation.go",
        "Bytes:      append([]byte(nil), page.Bytes...),\n\t\t\tProgrammed: page.Programmed,",
        "Bytes:      page.Bytes,\n\t\t\tProgrammed: page.Programmed,",
    ),
    (
        "omit-whole-media-program-marker",
        "TestObserveMediaMatchesEveryBlockThroughProgramAndEraseCuts",
        "ObserveMedia()=",
        "relocation.go",
        "Bytes:      append([]byte(nil), page.Bytes...),\n\t\t\tProgrammed: page.Programmed,",
        "Bytes:      append([]byte(nil), page.Bytes...),\n\t\t\tProgrammed: false,",
    ),
    (
        "omit-whole-media-erase-generation",
        "TestObserveMediaMatchesEveryBlockThroughProgramAndEraseCuts",
        "ObserveMedia()=",
        "relocation.go",
        "CompletedErases: progress.CompletedErases,\n\t\tIncompleteErase: progress.IncompleteErase,",
        "CompletedErases: 0,\n\t\tIncompleteErase: progress.IncompleteErase,",
    ),
    (
        "alias-reclamation-block-slice",
        "TestObserveReclamationMatchesRolesAndEveryLifecycleState",
        "mutating returned role/progress slices changed the device",
        "relocation.go",
        "// selection. In particular, the payload-bearing page slice is never copied.\n\tblocks := append([]BlockSnapshot(nil), device.blocks...)",
        "// selection. In particular, the payload-bearing page slice is never copied.\n\tblocks := device.blocks",
    ),
    (
        "lose-active-block-zero-bit",
        "TestObserveReclamationMatchesRolesAndEveryLifecycleState",
        "active block zero lost its explicit active bit",
        "relocation.go",
        "observation.Active = true",
        "observation.Active = device.evacuationBlock != 0",
    ),
    (
        "ignore-evacuation-start-caller-destination-fence",
        "TestBeginEvacuationExcludingMatchesLegacyAndIgnoresVictimBit",
        "external destination fence error=",
        "evacuation.go",
        "if blocked {\n\t\t\tmask[index] = true",
        "if blocked {\n\t\t\tmask[index] = false",
    ),
    (
        "reject-victim-bit-in-evacuation-start-mask",
        "TestBeginEvacuationExcludingMatchesLegacyAndIgnoresVictimBit",
        "unexpected error evacuation target block is excluded",
        "evacuation.go",
        "\tmask[int(block)] = false",
        "\tif len(excluded) != 0 && excluded[int(block)] { mask[int(block)] = true }",
    ),
    (
        "evacuation-active-precedes-mask-width",
        "TestBeginEvacuationExcludingValidationOrder",
        "mask width must precede erase budget",
        "evacuation.go",
        "if len(destinationExcluded) != 0 && len(destinationExcluded) != len(device.blocks) {\n\t\treturn device, ftlevacuationplan.ErrInvalidBlockMask",
        "if len(destinationExcluded) != 0 && len(destinationExcluded) != len(device.blocks) {\n\t\treturn device, ErrEvacuationActive",
    ),
    (
        "ignore-active-copy-caller-destination-fence",
        "TestCopyNextEvacuationPageExcludingUsesCurrentFenceAndExactReceipt",
        "fenced copy receipt=",
        "evacuation.go",
        "if blocked {\n\t\t\tmask[index] = true",
        "if blocked {\n\t\t\tmask[index] = false",
    ),
    (
        "respect-active-copy-victim-mask-bit",
        "TestCopyNextEvacuationPageExcludingBoundedMasks",
        "unexpected error evacuation target block is excluded",
        "evacuation.go",
        "\tmask[int(block)] = false",
        "\tif len(excluded) != 0 && excluded[int(block)] { mask[int(block)] = true }",
    ),
    (
        "copy-before-full-plan-capacity-check",
        "TestCopyNextEvacuationPageExcludingPreflightsEveryLiveSource",
        "error code = \"\", want \"insufficient_capacity\"",
        "ftl-evacuation-plan/plan.go",
        "if destinationIndex < len(plan.Copies) {\n\t\treturn Plan{}, ErrInsufficientCapacity\n\t}",
        "if false && destinationIndex < len(plan.Copies) {\n\t\treturn Plan{}, ErrInsufficientCapacity\n\t}",
    ),
    (
        "empty-target-precedes-copy-mask-validation",
        "TestCopyNextEvacuationPageExcludingValidationOrderAndEmptyTarget",
        "active target validates mask before no-live check",
        "evacuation.go",
        "if len(destinationExcluded) != 0 && len(destinationExcluded) != len(device.blocks) {\n\t\treturn device, EvacuationCopyReceipt{}, ftlevacuationplan.ErrInvalidBlockMask",
        "if len(destinationExcluded) != 0 && len(destinationExcluded) != len(device.blocks) {\n\t\treturn device, EvacuationCopyReceipt{}, ErrEvacuationNoLivePages",
    ),
    (
        "omit-incomplete-erase-progress",
        "TestObserveReclamationMatchesRolesAndEveryLifecycleState",
        "active incomplete erase was not captured in both fields",
        "relocation.go",
        "// selection. In particular, the payload-bearing page slice is never copied.\n\tblocks := append([]BlockSnapshot(nil), device.blocks...)",
        "// selection. In particular, the payload-bearing page slice is never copied.\n\tblocks := append([]BlockSnapshot(nil), device.blocks...)\n\tfor index := range blocks { blocks[index].IncompleteErase = false }",
    ),
    (
        "restore-accepts-wrong-block-width",
        "TestRestoreQuiescentValidationOrderAndZeroResults",
        "RestoreQuiescent() error=",
        "restore_quiescent.go",
        "if len(block.Pages) != int(pagesPerBlock) {",
        "if false {",
    ),
    (
        "restore-short-circuits-page-map-geometry-error",
        "TestRestoreQuiescentValidationOrderAndZeroResults",
        "RestoreQuiescent() error=",
        "restore_quiescent.go",
        "table, err := ftlpagemap.Restore(media.LogicalPages, physicalCount, snapshot)\n\tif err != nil {",
        "if physicalCount < 2 { return Device{}, ErrInvalidBlockGeometry }\n\ttable, err := ftlpagemap.Restore(media.LogicalPages, physicalCount, snapshot)\n\tif err != nil {",
    ),
    (
        "restore-accepts-incomplete-erase",
        "TestRestoreQuiescentValidationOrderAndZeroResults",
        "RestoreQuiescent() error=",
        "restore_quiescent.go",
        "if block.IncompleteErase {",
        "if false {",
    ),
    (
        "restore-accepts-programmed-erased-byte-mismatch",
        "TestRestoreQuiescentValidationOrderAndZeroResults",
        "RestoreQuiescent() error=",
        "restore_quiescent.go",
        "if !page.Programmed && !restorePageIsErased(page.Bytes) {",
        "if false {",
    ),
    (
        "restore-accepts-free-role-with-program-marker",
        "TestRestoreQuiescentValidationOrderAndZeroResults",
        "RestoreQuiescent() error=",
        "restore_quiescent.go",
        "if page.Programmed {\n\t\t\t\t\treturn Device{}, ErrMediaRoleMismatch",
        "if false {\n\t\t\t\t\treturn Device{}, ErrMediaRoleMismatch",
    ),
    (
        "restore-reuses-no-owner-identity",
        "TestRestoreQuiescentContinuesRealWriteCopyEraseHistory",
        "restored current owner",
        "restore_quiescent.go",
        "restored.ownerIdentities[logical] = &ownerPublicationIdentity{marker: 1}",
        "restored.ownerIdentities[logical] = nil",
    ),
    (
        "restore-admits-impossible-interrupted-prefix",
        "TestRestoreInterruptedValidationOrderAndZeroDevice",
        "restore = zero? false",
        "restore_quiescent.go",
        "if block.IncompleteErase && (!hasStale || minimumPrefix > maximumPrefix) {",
        "if block.IncompleteErase && (!hasStale || minimumPrefix > maximumPrefix && false) {",
    ),
    (
        "restore-drops-incomplete-block-fence",
        "TestRestoreInterruptedHistoryRetriesFencesAndReleases",
        "restored block lost its incomplete-erase fence",
        "restore_quiescent.go",
        "IncompleteErase: block.IncompleteErase,",
        "IncompleteErase: false,",
    ),
    (
        "shorter-erase-retry-prematurely-releases-progress",
        "TestRestoreInterruptedHistoryRetriesFencesAndReleases",
        "shorter retry lost earlier prefix progress",
        "relocation.go",
        "blocks[int(blockIndex)] = BlockSnapshot{CompletedErases: outcome.Block.CompletedErases, IncompleteErase: outcome.Block.IncompleteErase}",
        "blocks[int(blockIndex)] = BlockSnapshot{CompletedErases: outcome.Block.CompletedErases, IncompleteErase: outcome.Block.IncompleteErase && !block.IncompleteErase}",
    ),
    (
        "capture-skips-live-role-refusal",
        "TestCapturePrecedenceAndActiveEvacuation",
        "live/stale capture must refuse",
        "erase_intent.go",
        "default:\n\t\t\treturn ErrIneligibleBlock",
        "default:\n\t\t\tcontinue",
    ),
    (
        "capture-restarts-already-incomplete-erase",
        "TestCaptureSequenceBudgetIncompleteAndGeneration",
        "prior incomplete erase must be refused",
        "erase_intent.go",
        "if progress.IncompleteErase {",
        "if false {",
    ),
    (
        "capture-reverses-page-order",
        "TestCaptureEnumeratesEveryStaleFreeLayoutAndRequalifiesEraseCut",
        "stale mask",
        "erase_intent.go",
        "stale[int(physical-first)] = snapshot.PhysicalStatus[int(physical)] == ftlpagemap.Stale",
        "stale[len(stale)-1-int(physical-first)] = snapshot.PhysicalStatus[int(physical)] == ftlpagemap.Stale",
    ),
    (
        "accept-relocation-unpaired-surrogates",
        "TestRelocationFixtureRejectsInvalidUnicodeScalars",
        "ftl-nand-relocation reader accepted invalid Unicode scalar spelling",
        "fixture_test.go",
        "if err := validateSurrogateEscapes(raw); err != nil {\n\t\treturn err\n\t}",
        "if false { return validateSurrogateEscapes(raw) }",
    ),
    (
        "accept-block-reclamation-unpaired-surrogates",
        "TestBlockFixtureRejectsInvalidUnicodeScalars",
        "ftl-block-reclamation reader accepted invalid Unicode scalar spelling",
        "fixture_test.go",
        "if err := validateSurrogateEscapes(raw); err != nil {\n\t\treturn err\n\t}",
        "if false { return validateSurrogateEscapes(raw) }",
    ),
    (
        "accept-live-copy-unpaired-surrogates",
        "TestCopyFixtureRejectsInvalidUnicodeScalars",
        "ftl-live-page-copy reader accepted invalid Unicode scalar spelling",
        "fixture_test.go",
        "if err := validateSurrogateEscapes(raw); err != nil {\n\t\treturn err\n\t}",
        "if false { return validateSurrogateEscapes(raw) }",
    ),
    (
        "accept-evacuation-unpaired-surrogates",
        "TestEvacuationFixtureRejectsInvalidUnicodeScalars",
        "ftl-evacuation reader accepted invalid Unicode scalar spelling",
        "fixture_test.go",
        "if err := validateSurrogateEscapes(raw); err != nil {\n\t\treturn err\n\t}",
        "if false { return validateSurrogateEscapes(raw) }",
    ),
)

SCALAR_CLEAN_TESTS = (
    "TestRelocationFixtureRejectsInvalidUnicodeScalars",
    "TestBlockFixtureRejectsInvalidUnicodeScalars",
    "TestCopyFixtureRejectsInvalidUnicodeScalars",
    "TestEvacuationFixtureRejectsInvalidUnicodeScalars",
)


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, check=False, text=True, capture_output=True)


def copy_package(source: Path, destination: Path) -> None:
    destination.mkdir(parents=True)
    for path in source.glob("*.go"):
        shutil.copy2(path, destination / path.name)
    shutil.copy2(source / "go.mod", destination / "go.mod")


def materialize(root: Path) -> Path:
    package = root / "code/packages/go/ftl-nand-relocation"
    copy_package(PACKAGE, package)
    copy_package(GO_ROOT / "ftl-page-map", root / "code/packages/go/ftl-page-map")
    copy_package(GO_ROOT / "ftl-erase-role-evidence", root / "code/packages/go/ftl-erase-role-evidence")
    copy_package(GO_ROOT / "nand-page-program-cuts", root / "code/packages/go/nand-page-program-cuts")
    copy_package(GO_ROOT / "nand-block-erase-cuts", root / "code/packages/go/nand-block-erase-cuts")
    copy_package(GO_ROOT / "ftl-erase-victim", root / "code/packages/go/ftl-erase-victim")
    copy_package(GO_ROOT / "ftl-evacuation-plan", root / "code/packages/go/ftl-evacuation-plan")
    fixture_dir = root / "code/specs/fixtures/database-media-v1"
    fixture_dir.mkdir(parents=True)
    for fixture_name in FIXTURES:
        shutil.copy2(CODE_ROOT / "specs/fixtures/database-media-v1" / fixture_name, fixture_dir / fixture_name)
    return package


def mutate(package: Path, relative_file: str, old: str, replacement: str) -> None:
    source = package / relative_file
    if not source.exists():
        source = package.parent / relative_file
    content = source.read_text(encoding="utf-8")
    if content.count(old) != 1:
        raise RuntimeError(f"{relative_file}: anchor occurs {content.count(old)} times; expected once")
    source.write_text(content.replace(old, replacement, 1), encoding="utf-8")


def main() -> int:
    go = os.environ.get("GO", "go")
    selected = os.environ.get("MUTATION_FILTER")
    mutations = MUTATIONS
    if selected:
        names = set(selected.split(","))
        unknown = names - {mutation[0] for mutation in MUTATIONS}
        if unknown:
            raise RuntimeError(f"unknown mutation names: {', '.join(sorted(unknown))}")
        mutations = tuple(mutation for mutation in MUTATIONS if mutation[0] in names)
    with tempfile.TemporaryDirectory(prefix="ftl-nand-relocation-mutations-") as temporary:
        root = Path(temporary)
        clean_package = materialize(root / "clean")
        clean = run([go, "test", "-count=1", "./..."], clean_package)
        if clean.returncode != 0:
            print(clean.stdout, end="")
            print(clean.stderr, end="", file=os.sys.stderr)
            raise RuntimeError("clean control failed")
        print("clean control: pass")

        for test_name in SCALAR_CLEAN_TESTS:
            result = run([go, "test", "-count=1", "-v", "-run", f"^{test_name}$", "."], clean_package)
            output = result.stdout + result.stderr
            marker = f"--- PASS: {test_name}"
            if result.returncode != 0 or marker not in output:
                print(output, end="")
                raise RuntimeError(f"named scalar clean control failed: {test_name}")
            print(f"clean scalar control: {test_name}")

        for name, test_name, witness, relative_file, old, replacement in mutations:
            package = materialize(root / name)
            mutate(package, relative_file, old, replacement)
            result = run([go, "test", "-count=1", "-run", f"^{test_name}$", "."], package)
            output = result.stdout + result.stderr
            if result.returncode == 0:
                raise RuntimeError(f"mutant unexpectedly passed: {name}")
            if f"--- FAIL: {test_name}" not in output or witness not in output:
                print(output, end="")
                raise RuntimeError(f"mutant did not reach its intended witness: {name}")
            print(f"killed {name}: {witness}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
