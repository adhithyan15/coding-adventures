//! Decompressor configuration with security bounds and anti-bomb defenses.

/// Default maximum allowed uncompressed SWF container payload (256 MB).
pub const DEFAULT_MAX_UNCOMPRESSED_BYTES: usize = 256 * 1024 * 1024;

/// Default maximum expansion ratio threshold (50:1).
pub const DEFAULT_MAX_EXPANSION_RATIO: f64 = 50.0;

/// Byte threshold before expansion ratio checks engage (1 MB).
pub const EXPANSION_RATIO_THRESHOLD_BYTES: usize = 1024 * 1024;

/// Configuration options for SWF decompressors and parsers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwfDecompressorConfig {
    /// Maximum allowed uncompressed byte count.
    pub max_uncompressed_bytes: usize,
    /// Maximum expansion ratio permitted beyond 1 MB of decompressed data.
    pub max_expansion_ratio: f64,
}

impl Default for SwfDecompressorConfig {
    fn default() -> Self {
        Self::default_decompressor_config()
    }
}

impl SwfDecompressorConfig {
    /// Creates the default secure decompressor configuration (256 MB cap, 50:1 ratio limit).
    #[inline]
    pub const fn default_decompressor_config() -> Self {
        Self {
            max_uncompressed_bytes: DEFAULT_MAX_UNCOMPRESSED_BYTES,
            max_expansion_ratio: DEFAULT_MAX_EXPANSION_RATIO,
        }
    }

    /// Sets the maximum permitted uncompressed byte limit.
    #[inline]
    pub const fn with_max_uncompressed_bytes(mut self, max: usize) -> Self {
        self.max_uncompressed_bytes = max;
        self
    }

    /// Sets the maximum expansion ratio threshold.
    #[inline]
    pub const fn with_max_expansion_ratio(mut self, ratio: f64) -> Self {
        self.max_expansion_ratio = ratio;
        self
    }
}
