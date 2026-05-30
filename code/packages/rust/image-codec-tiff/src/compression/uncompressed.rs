// # uncompressed.rs — TIFF Uncompressed (Compression = 1)
//
// The simplest compression scheme: none at all. Strip bytes are raw pixel
// data, packed at BitsPerSample bits per channel, MSB-first within each byte,
// rows padded to byte boundaries.
//
// This is the codec to use when you want to understand the raw pixel layout
// before adding compression. It's also used for the encoder output.

/// Decompress an uncompressed TIFF strip.
///
/// Since there is no compression, this is just a clone of the input bytes.
/// The caller handles alignment and pixel parsing.
///
/// We return a `Vec<u8>` rather than a slice to keep the interface uniform
/// across all compression codecs — callers don't need to know whether the
/// bytes were already in memory or needed decoding.
pub fn decompress(data: &[u8]) -> Vec<u8> {
    data.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompress_empty() {
        assert_eq!(decompress(&[]), Vec::<u8>::new());
    }

    #[test]
    fn decompress_returns_same_bytes() {
        let data = vec![0x01u8, 0x80, 0xFF, 0x42];
        assert_eq!(decompress(&data), data);
    }

    #[test]
    fn decompress_does_not_alias_input() {
        let data = vec![1u8, 2, 3];
        let mut result = decompress(&data);
        result[0] = 99;
        assert_eq!(data[0], 1); // original unchanged
    }
}
