/// Capability manifests, taxonomy, and target matching.
///
/// The Dart lane of `capability-cage` (slice 1): the [Category]/[Action]
/// taxonomy with [isValidCombination], [Capability] declarations,
/// [Manifest] loading (`required_capabilities.json`) with schema, pair,
/// and read/write-separation validation, [matchTarget] glob matching, and
/// [CapabilitySurfaceSummary] counting.
///
/// Slice 2 adds the audit envelopes ([OperationRecord], [Operation],
/// [AuditSink]) and the [Backend] seam ([OpenBackend], [DenyAllBackend],
/// [TestBackend], [runWithBackend]). Slice 3 adds the secure wrappers
/// ([readFile], [writeFile], [createFile], [deleteFile], [listDir],
/// [readVar], [writeVar]).
library coding_adventures_capability_cage;

export 'src/audit.dart';
export 'src/backend.dart';
export 'src/capability.dart';
export 'src/category.dart';
export 'src/errors.dart';
export 'src/glob.dart';
export 'src/manifest.dart';
export 'src/secure_env.dart';
export 'src/secure_file.dart';
