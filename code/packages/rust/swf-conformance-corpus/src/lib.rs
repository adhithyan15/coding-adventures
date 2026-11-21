//! # SWF Conformance Corpus (`SWF08`)
//!
//! Clean-room synthetic `.swf` binary encoder (`SwfBitWriter` + `SwfBuilder`),
//! deterministic conformance corpus runner (`CorpusRunner`), pinned canonical SHA-256
//! frame & audio digest verifier (`verify_corpus_digests`), and adversarial security
//! corpus tests (`SWF00` §3.8, §7–§9).
//!
//! ## Mandatory Security & Capability Policy
//!
//! - **`CAP001` / `CAP002`**: `#![forbid(unsafe_code)]`, `"capabilities": []`.
//! - **Bounded Bit/Byte Encoding**: `MAX_CORPUS_SWF_BYTES = 16 MiB`, `MAX_CORPUS_FRAMES = 1024`,
//!   `MAX_BIT_WIDTH = 31`, checked bit shifts and `try_from` integer conversions.

#![forbid(unsafe_code)]

pub mod bit_writer;
pub mod builder;
pub mod capabilities;
pub mod corpus;
pub mod error;

pub use bit_writer::{min_signed_bit_width, min_unsigned_bit_width, SwfBitWriter};
pub use builder::{
    encode_adpcm_stream_block_tag, encode_define_shape_tag, encode_define_sound_pcm16_tag,
    encode_define_sprite_tag, encode_do_abc_tag, encode_do_action_tag, encode_place_object_tag,
    encode_remove_object2_tag, encode_set_background_color_tag, encode_sound_stream_head2_adpcm,
    encode_start_sound_tag, encode_swf_tag, write_abc_u30, Avm1ActionBuilder, PlaceTagSpec,
    ShapeDefSpec, ShapeFillSpec, ShapeLineSpec, ShapeRecordSpec, SwfBuilder, SwfCompression,
};
pub use capabilities::PureCapabilities;
pub use corpus::{
    corpus_pcm_digest_hex, corpus_scene_digest_hex, run_all_corpus_cases, run_corpus_case,
    verify_corpus_digests, AdversarialCaseId, AdversarialOutcome, CorpusCaseReport, CorpusEntry,
    CorpusId, CorpusRunner,
};
pub use error::{
    CorpusError, MAX_BIT_WIDTH, MAX_BUILDER_TAGS, MAX_CORPUS_FRAMES, MAX_CORPUS_SWF_BYTES,
};

// Re-exports from SWF07 for downstream consumers
pub use flash_player_host::{
    compute_pcm_digest_hex, compute_scene_digest_hex, run_headless_swf, verify_headless_digests,
    FlashPlayer, FlashPlayerConfig, HeadlessFrameDigest, HeadlessRunConfig, HeadlessRunReport,
};

/// Validates that an SWF byte size is within `1..=MAX_CORPUS_SWF_BYTES`.
#[inline]
pub const fn validate_corpus_swf_bytes(len: usize) -> Result<bool, CorpusError> {
    if len >= 1 && len <= MAX_CORPUS_SWF_BYTES {
        Ok(true)
    } else {
        Err(CorpusError::SwfSizeExceeded)
    }
}

/// Validates that a corpus frame count is within `1..=MAX_CORPUS_FRAMES`.
#[inline]
pub const fn validate_corpus_frame_count(frames: usize) -> Result<bool, CorpusError> {
    if frames >= 1 && frames <= MAX_CORPUS_FRAMES {
        Ok(true)
    } else {
        Err(CorpusError::FrameCountExceeded)
    }
}

/// Validates that a bit field width does not exceed `MAX_BIT_WIDTH` (`31`).
#[inline]
pub const fn validate_corpus_bit_width(bits: usize) -> Result<bool, CorpusError> {
    if bits <= MAX_BIT_WIDTH as usize {
        Ok(true)
    } else {
        Err(CorpusError::BitWidthExceeded)
    }
}

/// Validates that `bytes` is a 64-byte ASCII hexadecimal SHA-256 digest without allocating.
#[inline]
pub const fn validate_corpus_digest_hex(bytes: &[u8]) -> Result<bool, CorpusError> {
    if bytes.len() != 64 {
        return Err(CorpusError::InvalidDigestFormat);
    }
    let mut i = 0usize;
    while i < 64 {
        if !bytes[i].is_ascii_hexdigit() {
            return Err(CorpusError::InvalidDigestFormat);
        }
        i += 1;
    }
    Ok(true)
}
