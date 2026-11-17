//! SWF Tag framing and zero-allocation tag streaming iterator.

use crate::error::SwfError;

/// Enumeration of known SWF Tag kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwfTagKind {
    /// Tag 0: End of timeline/tag stream.
    End,
    /// Tag 1: Show current frame.
    ShowFrame,
    /// Tag 2: Define vector shape (version 1).
    DefineShape,
    /// Tag 4: Place display object (version 1).
    PlaceObject,
    /// Tag 5: Remove display object (version 1).
    RemoveObject,
    /// Tag 6: Define JPEG image bits.
    DefineBits,
    /// Tag 9: Set stage background RGB color.
    SetBackgroundColor,
    /// Tag 12: ActionScript bytecode block for timeline frame.
    DoAction,
    /// Tag 14: Define event sound.
    DefineSound,
    /// Tag 15: Start sound playback.
    StartSound,
    /// Tag 18: Sound stream configuration header (version 1).
    SoundStreamHead,
    /// Tag 19: Streaming sound block for frame.
    SoundStreamBlock,
    /// Tag 20: Lossless zlib-compressed bitmap data (version 1).
    DefineBitsLossless,
    /// Tag 21: JPEG image bits and tables combined.
    DefineBitsJPEG2,
    /// Tag 22: Define vector shape (version 2).
    DefineShape2,
    /// Tag 26: Place display object with matrix/cxform/ratio/clip (version 2).
    PlaceObject2,
    /// Tag 28: Remove display object (version 2).
    RemoveObject2,
    /// Tag 32: Define vector shape with RGBA styles (version 3).
    DefineShape3,
    /// Tag 35: JPEG image bits with alpha mask stream.
    DefineBitsJPEG3,
    /// Tag 36: Lossless zlib-compressed RGBA bitmap data (version 2).
    DefineBitsLossless2,
    /// Tag 39: Define MovieClip sprite timeline container.
    DefineSprite,
    /// Tag 45: Sound stream configuration header (version 2).
    SoundStreamHead2,
    /// Tag 59: ActionScript initialization script for movieclip.
    DoInitAction,
    /// Tag 69: Global file attributes (AS3, Network, DirectBlit).
    FileAttributes,
    /// Tag 70: Place display object with blend modes/filters (version 3).
    PlaceObject3,
    /// Tag 76: ActionScript 3 symbol class linkages.
    SymbolClass,
    /// Tag 77: XMP / RDF movie metadata.
    Metadata,
    /// Tag 82: ActionScript 3 bytecode definition (DoABC).
    DoABC,
    /// Tag 83: Define vector shape with stroke scaling / cap styles (version 4).
    DefineShape4,
    /// Tag 90: JPEG with deblocking filter parameter.
    DefineBitsJPEG4,
    /// Any unknown or unhandled tag code.
    Unknown(u16),
}

impl SwfTagKind {
    /// Maps a 10-bit integer tag code to its `SwfTagKind`.
    pub fn from_code(code: u16) -> Self {
        match code {
            0 => Self::End,
            1 => Self::ShowFrame,
            2 => Self::DefineShape,
            4 => Self::PlaceObject,
            5 => Self::RemoveObject,
            6 => Self::DefineBits,
            9 => Self::SetBackgroundColor,
            12 => Self::DoAction,
            14 => Self::DefineSound,
            15 => Self::StartSound,
            18 => Self::SoundStreamHead,
            19 => Self::SoundStreamBlock,
            20 => Self::DefineBitsLossless,
            21 => Self::DefineBitsJPEG2,
            22 => Self::DefineShape2,
            26 => Self::PlaceObject2,
            28 => Self::RemoveObject2,
            32 => Self::DefineShape3,
            35 => Self::DefineBitsJPEG3,
            36 => Self::DefineBitsLossless2,
            39 => Self::DefineSprite,
            45 => Self::SoundStreamHead2,
            59 => Self::DoInitAction,
            69 => Self::FileAttributes,
            70 => Self::PlaceObject3,
            76 => Self::SymbolClass,
            77 => Self::Metadata,
            82 => Self::DoABC,
            83 => Self::DefineShape4,
            90 => Self::DefineBitsJPEG4,
            other => Self::Unknown(other),
        }
    }

    /// Numeric code of this tag kind.
    pub fn tag_code(&self) -> u16 {
        match self {
            Self::End => 0,
            Self::ShowFrame => 1,
            Self::DefineShape => 2,
            Self::PlaceObject => 4,
            Self::RemoveObject => 5,
            Self::DefineBits => 6,
            Self::SetBackgroundColor => 9,
            Self::DoAction => 12,
            Self::DefineSound => 14,
            Self::StartSound => 15,
            Self::SoundStreamHead => 18,
            Self::SoundStreamBlock => 19,
            Self::DefineBitsLossless => 20,
            Self::DefineBitsJPEG2 => 21,
            Self::DefineShape2 => 22,
            Self::PlaceObject2 => 26,
            Self::RemoveObject2 => 28,
            Self::DefineShape3 => 32,
            Self::DefineBitsJPEG3 => 35,
            Self::DefineBitsLossless2 => 36,
            Self::DefineSprite => 39,
            Self::SoundStreamHead2 => 45,
            Self::DoInitAction => 59,
            Self::FileAttributes => 69,
            Self::PlaceObject3 => 70,
            Self::SymbolClass => 76,
            Self::Metadata => 77,
            Self::DoABC => 82,
            Self::DefineShape4 => 83,
            Self::DefineBitsJPEG4 => 90,
            Self::Unknown(c) => *c,
        }
    }

    /// Descriptive name of the tag kind.
    pub fn tag_name(&self) -> &'static str {
        match self {
            Self::End => "End",
            Self::ShowFrame => "ShowFrame",
            Self::DefineShape => "DefineShape",
            Self::PlaceObject => "PlaceObject",
            Self::RemoveObject => "RemoveObject",
            Self::DefineBits => "DefineBits",
            Self::SetBackgroundColor => "SetBackgroundColor",
            Self::DoAction => "DoAction",
            Self::DefineSound => "DefineSound",
            Self::StartSound => "StartSound",
            Self::SoundStreamHead => "SoundStreamHead",
            Self::SoundStreamBlock => "SoundStreamBlock",
            Self::DefineBitsLossless => "DefineBitsLossless",
            Self::DefineBitsJPEG2 => "DefineBitsJPEG2",
            Self::DefineShape2 => "DefineShape2",
            Self::PlaceObject2 => "PlaceObject2",
            Self::RemoveObject2 => "RemoveObject2",
            Self::DefineShape3 => "DefineShape3",
            Self::DefineBitsJPEG3 => "DefineBitsJPEG3",
            Self::DefineBitsLossless2 => "DefineBitsLossless2",
            Self::DefineSprite => "DefineSprite",
            Self::SoundStreamHead2 => "SoundStreamHead2",
            Self::DoInitAction => "DoInitAction",
            Self::FileAttributes => "FileAttributes",
            Self::PlaceObject3 => "PlaceObject3",
            Self::SymbolClass => "SymbolClass",
            Self::Metadata => "Metadata",
            Self::DoABC => "DoABC",
            Self::DefineShape4 => "DefineShape4",
            Self::DefineBitsJPEG4 => "DefineBitsJPEG4",
            Self::Unknown(_) => "Unknown",
        }
    }
}

/// Header of an SWF tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwfTagHeader {
    /// Tag kind.
    pub kind: SwfTagKind,
    /// Length of the tag payload in bytes.
    pub length: usize,
    /// Whether this tag used long framing (length == 0x3F + 4 bytes UI32).
    pub is_long: bool,
}

impl SwfTagHeader {
    /// Returns true if this tag is the terminal `End` tag (code 0).
    #[inline]
    pub fn is_end_tag(&self) -> bool {
        matches!(self.kind, SwfTagKind::End)
    }

    /// Payload length in bytes.
    #[inline]
    pub fn tag_payload_len(&self) -> usize {
        self.length
    }
}

/// A borrowed SWF tag holding a bounded sub-slice to its payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwfTag<'a> {
    /// Header specifying tag kind and length.
    pub header: SwfTagHeader,
    /// Bounded sub-slice containing the tag payload bytes.
    pub data: &'a [u8],
}

impl<'a> SwfTag<'a> {
    /// Reference to the tag header.
    #[inline]
    pub fn tag_header_ref(&self) -> &SwfTagHeader {
        &self.header
    }

    /// Borrowed slice of the tag payload.
    #[inline]
    pub fn tag_payload_slice(&self) -> &'a [u8] {
        self.data
    }
}

/// Zero-allocation streaming iterator over uncompressed SWF body tags.
#[derive(Debug, Clone)]
pub struct SwfTagIter<'a> {
    data: &'a [u8],
    offset: usize,
    finished: bool,
}

impl<'a> SwfTagIter<'a> {
    /// Creates a tag iterator over uncompressed tag stream bytes.
    #[inline]
    pub const fn from_payload(payload: &'a [u8]) -> Self {
        Self {
            data: payload,
            offset: 0,
            finished: false,
        }
    }

    /// Number of unprocessed bytes remaining in the stream.
    #[inline]
    pub fn iter_remaining_bytes(&self) -> usize {
        self.data.len().saturating_sub(self.offset)
    }
}

impl<'a> Iterator for SwfTagIter<'a> {
    type Item = Result<SwfTag<'a>, SwfError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished || self.offset >= self.data.len() {
            return None;
        }

        if self.offset + 2 > self.data.len() {
            self.finished = true;
            return Some(Err(SwfError::UnexpectedEof));
        }

        let b0 = self.data[self.offset];
        let b1 = self.data[self.offset + 1];
        let code_and_len = u16::from_le_bytes([b0, b1]);
        let tag_code = code_and_len >> 6;
        let short_len = (code_and_len & 0x3F) as usize;

        let mut header_len = 2;
        let (payload_len, is_long) = if short_len == 0x3F {
            if self.offset + 6 > self.data.len() {
                self.finished = true;
                return Some(Err(SwfError::UnexpectedEof));
            }
            let raw_len = i32::from_le_bytes([
                self.data[self.offset + 2],
                self.data[self.offset + 3],
                self.data[self.offset + 4],
                self.data[self.offset + 5],
            ]);
            if raw_len < 0 {
                self.finished = true;
                return Some(Err(SwfError::InvalidTagLength));
            }
            header_len = 6;
            (raw_len as usize, true)
        } else {
            (short_len, false)
        };

        let payload_start = self.offset + header_len;
        let payload_end = match payload_start.checked_add(payload_len) {
            Some(end) => end,
            None => {
                self.finished = true;
                return Some(Err(SwfError::InvalidTagLength));
            }
        };

        if payload_end > self.data.len() {
            self.finished = true;
            return Some(Err(SwfError::UnexpectedEof));
        }

        let kind = SwfTagKind::from_code(tag_code);
        let tag = SwfTag {
            header: SwfTagHeader {
                kind,
                length: payload_len,
                is_long,
            },
            data: &self.data[payload_start..payload_end],
        };

        self.offset = payload_end;
        if tag.header.is_end_tag() {
            self.finished = true;
        }

        Some(Ok(tag))
    }
}
