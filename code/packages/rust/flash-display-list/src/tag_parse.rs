//! Parsing and representation of SWF display list depth tags.
//!
//! Covers:
//! - `PlaceObject` (Tag 4)
//! - `RemoveObject` (Tag 5)
//! - `PlaceObject2` (Tag 26)
//! - `RemoveObject2` (Tag 28)
//! - `PlaceObject3` (Tag 70)

use swf_container::{BitReader, SwfColorTransform, SwfMatrix};

use crate::error::DisplayListError;

/// Maximum byte length for null-terminated instance or class names (CWE-400 guard).
const MAX_TAG_CSTRING_BYTES: usize = 4096;

fn read_bounded_cstring(reader: &mut BitReader<'_>) -> Result<String, DisplayListError> {
    let mut name_bytes = Vec::new();
    loop {
        let b = reader.read_ub(8)? as u8;
        if b == 0 {
            break;
        }
        if name_bytes.len() >= MAX_TAG_CSTRING_BYTES {
            return Err(DisplayListError::CorruptedData);
        }
        name_bytes.push(b);
    }
    Ok(String::from_utf8_lossy(&name_bytes).to_string())
}

/// Parsed payload of a `PlaceObject`, `PlaceObject2`, or `PlaceObject3` tag.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaceObjectRecord {
    /// Target depth layer.
    pub depth: i32,
    /// Library character ID to place or replace.
    pub character_id: Option<u16>,
    /// Affine transformation matrix if updated.
    pub matrix: Option<SwfMatrix>,
    /// Color multiplier / additive transform if updated.
    pub color_transform: Option<SwfColorTransform>,
    /// Tween / morph ratio (0..=65535).
    pub ratio: Option<u16>,
    /// Instance name for ActionScript referencing.
    pub name: Option<String>,
    /// Stencil clipping depth ceiling.
    pub clip_depth: Option<i32>,
    /// Blend mode index (SWF 8+).
    pub blend_mode: Option<u8>,
    /// Visibility flag (SWF 8+).
    pub visible: Option<bool>,
    /// Whether this tag updates an existing display object (`has_move`).
    pub is_move: bool,
}

impl PlaceObjectRecord {
    /// Returns the target depth layer.
    #[inline]
    pub fn place_depth_slot(&self) -> i32 {
        self.depth
    }

    /// Returns the character identifier if present.
    #[inline]
    pub fn place_character_identifier(&self) -> Option<u16> {
        self.character_id
    }

    /// Returns the affine matrix transformation if present.
    #[inline]
    pub fn place_matrix_transform(&self) -> Option<SwfMatrix> {
        self.matrix
    }

    /// Returns the color transform if present.
    #[inline]
    pub fn place_cxform_transform(&self) -> Option<SwfColorTransform> {
        self.color_transform
    }

    /// Returns the tween/morph ratio if present.
    #[inline]
    pub fn place_ratio_value(&self) -> Option<u16> {
        self.ratio
    }

    /// Returns the instance name if present.
    #[inline]
    pub fn place_instance_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the clipping mask depth limit if present.
    #[inline]
    pub fn place_stencil_depth(&self) -> Option<i32> {
        self.clip_depth
    }

    /// Returns the blend mode if present.
    #[inline]
    pub fn place_blend_mode_value(&self) -> Option<u8> {
        self.blend_mode
    }

    /// Returns the visibility flag if present.
    #[inline]
    pub fn place_visibility_flag(&self) -> Option<bool> {
        self.visible
    }

    /// Returns whether this record represents a modification (`has_move`).
    #[inline]
    pub fn place_is_move_flag(&self) -> bool {
        self.is_move
    }
}

/// Parsed payload of a `RemoveObject` or `RemoveObject2` tag.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveObjectRecord {
    /// Depth slot to remove.
    pub depth: i32,
    /// Character ID to confirm removal (present in `RemoveObject`, `None` in `RemoveObject2`).
    pub character_id: Option<u16>,
}

impl RemoveObjectRecord {
    /// Returns the depth slot to remove.
    #[inline]
    pub fn remove_depth_slot(&self) -> i32 {
        self.depth
    }

    /// Returns the character ID if specified.
    #[inline]
    pub fn remove_character_identifier(&self) -> Option<u16> {
        self.character_id
    }
}

/// High-level display list depth tag representation.
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayListTag {
    /// Placement or modification tag (`PlaceObject`, `PlaceObject2`, `PlaceObject3`).
    Place(PlaceObjectRecord),
    /// Removal tag (`RemoveObject`, `RemoveObject2`).
    Remove(RemoveObjectRecord),
}

/// Parses a legacy `PlaceObject` (Tag 4) payload.
pub fn parse_place_object(bytes: &[u8]) -> Result<PlaceObjectRecord, DisplayListError> {
    if bytes.len() < 4 {
        return Err(DisplayListError::UnexpectedEof);
    }
    let character_id = u16::from_le_bytes([bytes[0], bytes[1]]);
    let depth = u16::from_le_bytes([bytes[2], bytes[3]]) as i32;

    let mut reader = BitReader::from_bits(&bytes[4..]);
    let matrix = reader.read_matrix()?;

    let color_transform = if reader.bits_remaining() > 0 {
        Some(reader.read_cxform()?)
    } else {
        None
    };

    Ok(PlaceObjectRecord {
        depth,
        character_id: Some(character_id),
        matrix: Some(matrix),
        color_transform,
        ratio: None,
        name: None,
        clip_depth: None,
        blend_mode: None,
        visible: None,
        is_move: false,
    })
}

/// Parses a `PlaceObject2` (Tag 26) payload.
pub fn parse_place_object2(bytes: &[u8]) -> Result<PlaceObjectRecord, DisplayListError> {
    if bytes.len() < 3 {
        return Err(DisplayListError::UnexpectedEof);
    }
    let flags = bytes[0];
    let depth = u16::from_le_bytes([bytes[1], bytes[2]]) as i32;

    let has_move = (flags & 0x01) != 0;
    let has_character = (flags & 0x02) != 0;
    let has_matrix = (flags & 0x04) != 0;
    let has_color_transform = (flags & 0x08) != 0;
    let has_ratio = (flags & 0x10) != 0;
    let has_name = (flags & 0x20) != 0;
    let has_clip_depth = (flags & 0x40) != 0;
    let has_clip_actions = (flags & 0x80) != 0;
    let _ = has_clip_actions;

    let mut reader = BitReader::from_bits(&bytes[3..]);

    let character_id = if has_character {
        let b0 = reader.read_ub(8)? as u8;
        let b1 = reader.read_ub(8)? as u8;
        Some(u16::from_le_bytes([b0, b1]))
    } else {
        None
    };

    let matrix = if has_matrix {
        Some(reader.read_matrix()?)
    } else {
        None
    };

    let color_transform = if has_color_transform {
        Some(reader.read_cxform_with_alpha()?)
    } else {
        None
    };

    let ratio = if has_ratio {
        let b0 = reader.read_ub(8)? as u8;
        let b1 = reader.read_ub(8)? as u8;
        Some(u16::from_le_bytes([b0, b1]))
    } else {
        None
    };

    let name = if has_name {
        Some(read_bounded_cstring(&mut reader)?)
    } else {
        None
    };

    let clip_depth = if has_clip_depth {
        let b0 = reader.read_ub(8)? as u8;
        let b1 = reader.read_ub(8)? as u8;
        Some(u16::from_le_bytes([b0, b1]) as i32)
    } else {
        None
    };

    Ok(PlaceObjectRecord {
        depth,
        character_id,
        matrix,
        color_transform,
        ratio,
        name,
        clip_depth,
        blend_mode: None,
        visible: None,
        is_move: has_move,
    })
}

/// Parses a `PlaceObject3` (Tag 70) payload.
pub fn parse_place_object3(bytes: &[u8]) -> Result<PlaceObjectRecord, DisplayListError> {
    if bytes.len() < 4 {
        return Err(DisplayListError::UnexpectedEof);
    }
    let flags1 = bytes[0];
    let flags2 = bytes[1];
    let depth = u16::from_le_bytes([bytes[2], bytes[3]]) as i32;

    let has_move = (flags1 & 0x01) != 0;
    let has_character = (flags1 & 0x02) != 0;
    let has_matrix = (flags1 & 0x04) != 0;
    let has_color_transform = (flags1 & 0x08) != 0;
    let has_ratio = (flags1 & 0x10) != 0;
    let has_name = (flags1 & 0x20) != 0;
    let has_clip_depth = (flags1 & 0x40) != 0;
    let has_clip_actions = (flags1 & 0x80) != 0;
    let _ = has_clip_actions;

    let has_filter_list = (flags2 & 0x01) != 0;
    let has_blend_mode = (flags2 & 0x02) != 0;
    let has_cache_as_bitmap = (flags2 & 0x04) != 0;
    let has_class_name = (flags2 & 0x08) != 0;
    let has_image = (flags2 & 0x10) != 0;
    let has_visible = (flags2 & 0x20) != 0;
    let has_opaque_background = (flags2 & 0x40) != 0;
    let _ = (has_cache_as_bitmap, has_opaque_background);

    let mut reader = BitReader::from_bits(&bytes[4..]);

    let _class_name = if has_class_name || (has_image && !has_character) {
        Some(read_bounded_cstring(&mut reader)?)
    } else {
        None
    };

    let character_id = if has_character {
        let b0 = reader.read_ub(8)? as u8;
        let b1 = reader.read_ub(8)? as u8;
        Some(u16::from_le_bytes([b0, b1]))
    } else {
        None
    };

    let matrix = if has_matrix {
        Some(reader.read_matrix()?)
    } else {
        None
    };

    let color_transform = if has_color_transform {
        Some(reader.read_cxform_with_alpha()?)
    } else {
        None
    };

    let ratio = if has_ratio {
        let b0 = reader.read_ub(8)? as u8;
        let b1 = reader.read_ub(8)? as u8;
        Some(u16::from_le_bytes([b0, b1]))
    } else {
        None
    };

    let name = if has_name {
        Some(read_bounded_cstring(&mut reader)?)
    } else {
        None
    };

    let clip_depth = if has_clip_depth {
        let b0 = reader.read_ub(8)? as u8;
        let b1 = reader.read_ub(8)? as u8;
        Some(u16::from_le_bytes([b0, b1]) as i32)
    } else {
        None
    };

    if has_filter_list {
        let num_filters = reader.read_ub(8)? as usize;
        for _ in 0..num_filters {
            let filter_id = reader.read_ub(8)? as u8;
            // Skip known filter payload sizes
            match filter_id {
                0 => {
                    // DropShadow
                    for _ in 0..23 {
                        reader.read_ub(8)?;
                    }
                }
                1 => {
                    // Blur
                    for _ in 0..9 {
                        reader.read_ub(8)?;
                    }
                }
                2 => {
                    // Glow
                    for _ in 0..15 {
                        reader.read_ub(8)?;
                    }
                }
                _ => break,
            }
        }
    }

    let blend_mode = if has_blend_mode {
        Some(reader.read_ub(8)? as u8)
    } else {
        None
    };

    let visible = if has_visible {
        Some(reader.read_ub(8)? != 0)
    } else {
        None
    };

    Ok(PlaceObjectRecord {
        depth,
        character_id,
        matrix,
        color_transform,
        ratio,
        name,
        clip_depth,
        blend_mode,
        visible,
        is_move: has_move,
    })
}

/// Parses a legacy `RemoveObject` (Tag 5) payload.
pub fn parse_remove_object(bytes: &[u8]) -> Result<RemoveObjectRecord, DisplayListError> {
    if bytes.len() < 4 {
        return Err(DisplayListError::UnexpectedEof);
    }
    let character_id = u16::from_le_bytes([bytes[0], bytes[1]]);
    let depth = u16::from_le_bytes([bytes[2], bytes[3]]) as i32;

    Ok(RemoveObjectRecord {
        depth,
        character_id: Some(character_id),
    })
}

/// Parses a `RemoveObject2` (Tag 28) payload.
pub fn parse_remove_object2(bytes: &[u8]) -> Result<RemoveObjectRecord, DisplayListError> {
    if bytes.len() < 2 {
        return Err(DisplayListError::UnexpectedEof);
    }
    let depth = u16::from_le_bytes([bytes[0], bytes[1]]) as i32;

    Ok(RemoveObjectRecord {
        depth,
        character_id: None,
    })
}

/// Parses any display list depth tag by SWF tag code.
pub fn parse_display_list_tag(
    tag_code: u16,
    payload: &[u8],
) -> Result<DisplayListTag, DisplayListError> {
    match tag_code {
        4 => parse_place_object(payload).map(DisplayListTag::Place),
        26 => parse_place_object2(payload).map(DisplayListTag::Place),
        70 => parse_place_object3(payload).map(DisplayListTag::Place),
        5 => parse_remove_object(payload).map(DisplayListTag::Remove),
        28 => parse_remove_object2(payload).map(DisplayListTag::Remove),
        _ => Err(DisplayListError::CorruptedData),
    }
}
