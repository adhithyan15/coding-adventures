//! `KernelCacheKey` — content-based cache key for the executor's
//! kernel cache.
//!
//! When the runtime asks an executor to `PrepareKernel`, the executor
//! caches the compiled artifact by a hash of the source.  If the
//! same source comes through later, the executor can return
//! `KernelReady` without re-compiling.
//!
//! This crate uses `std::collections::hash_map::DefaultHasher` (which
//! is SipHash) — a good general-purpose 64-bit hash that's already in
//! `std`.  No external hashing crate.

use crate::messages::KernelSource;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 64-bit content hash of a [`KernelSource`].  Used as a key in the
/// executor's kernel cache.
///
/// Two `KernelSource` values that compare equal hash to the same key
/// (per `Hash`'s contract); two that differ are extremely unlikely to
/// collide (the probability is ~2^-32 for a million distinct
/// kernels).  Collision is an availability concern (one extra
/// recompile), not a correctness one — backends always re-derive
/// the artifact from the supplied source if they don't trust the
/// cache.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct KernelCacheKey(pub u64);

impl KernelCacheKey {
    /// Compute the cache key for the given kernel source.  Includes
    /// the variant tag in the hash, so an MSL "void main()" doesn't
    /// collide with a CUDA "void main()".
    pub fn of(source: &KernelSource) -> KernelCacheKey {
        let mut h = DefaultHasher::new();
        source.wire_tag().hash(&mut h);
        match source {
            KernelSource::Msl { code, entry }
            | KernelSource::CudaC { code, entry }
            | KernelSource::Glsl { code, entry }
            | KernelSource::Wgsl { code, entry }
            | KernelSource::OpenClC { code, entry } => {
                code.hash(&mut h);
                entry.hash(&mut h);
            }
            KernelSource::SpirV { bytes, entry } => {
                bytes.hash(&mut h);
                entry.hash(&mut h);
            }
            KernelSource::Native { backend, blob } => {
                backend.hash(&mut h);
                blob.hash(&mut h);
            }
        }
        KernelCacheKey(h.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_sources_hash_equal() {
        let a = KernelSource::Msl {
            code: "kernel void k() {}".to_string(),
            entry: "k".to_string(),
        };
        let b = KernelSource::Msl {
            code: "kernel void k() {}".to_string(),
            entry: "k".to_string(),
        };
        assert_eq!(KernelCacheKey::of(&a), KernelCacheKey::of(&b));
    }

    #[test]
    fn different_code_hashes_differ() {
        let a = KernelSource::Msl {
            code: "kernel void a() {}".to_string(),
            entry: "a".to_string(),
        };
        let b = KernelSource::Msl {
            code: "kernel void b() {}".to_string(),
            entry: "b".to_string(),
        };
        assert_ne!(KernelCacheKey::of(&a), KernelCacheKey::of(&b));
    }

    #[test]
    fn variant_tag_in_hash_avoids_cross_lang_collision() {
        // Same code text, different language → different keys.
        let msl = KernelSource::Msl {
            code: "void main() {}".to_string(),
            entry: "main".to_string(),
        };
        let cuda = KernelSource::CudaC {
            code: "void main() {}".to_string(),
            entry: "main".to_string(),
        };
        assert_ne!(KernelCacheKey::of(&msl), KernelCacheKey::of(&cuda));
    }
}
