/// Manifest-level read/write separation classifier.
///
/// The Dart lane of `read-write-separation`: [Capability] declarations with
/// [CapabilityFlavor]/[CapabilityTrust] annotations, per-capability
/// [classifyCapability], manifest [summarizeManifest], and [validateManifest]
/// (throwing [RwsViolation] on untrusted-input/actuation mixing or
/// read/write resource overlap).
library coding_adventures_read_write_separation;

export 'src/read_write_separation.dart';
