//! Timeline state machine and depth-sorted child container (`MovieClip`).

use std::collections::{BTreeMap, HashMap};
use swf_container::{SwfColorTransform, SwfMatrix};

use crate::display_object::{DisplayObject, DisplaySlot};
use crate::error::DisplayListError;
use crate::tag_parse::{PlaceObjectRecord, RemoveObjectRecord};

/// Maximum nesting depth allowed in the display object hierarchy (CWE-400 / CWE-674).
pub const MAX_HIERARCHY_DEPTH: usize = 64;

/// Minimum allowed Flash display depth layer.
pub const MIN_DEPTH: i32 = -16384;

/// Maximum allowed Flash display depth layer.
pub const MAX_DEPTH: i32 = 65535;

/// Playback execution state of a MovieClip timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    /// Timeline advances automatically on each frame tick.
    Play,
    /// Timeline remains halted at current frame index.
    Stop,
}

/// Target frame identifier for timeline jumps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameTarget {
    /// 1-based numeric frame index.
    Index(u32),
    /// Named frame label string.
    Label(String),
}

/// A self-contained timeline entity with depth-ordered display slots and frame state.
#[derive(Debug, Clone, PartialEq)]
pub struct MovieClip {
    /// Current 1-based frame index (`1..=total_frames`).
    pub current_frame: u32,
    /// Total number of timeline frames (minimum 1).
    pub total_frames: u32,
    /// Playback state.
    pub play_state: PlayState,
    /// Mapping of frame label strings to 1-based frame numbers.
    pub frame_labels: HashMap<String, u32>,
    /// Depth-sorted display slots.
    pub slots: BTreeMap<i32, DisplaySlot>,
    /// Affine coordinate transform.
    pub transform: SwfMatrix,
    /// Color multiplier / additive transform.
    pub color_transform: SwfColorTransform,
    /// Stencil clipping depth ceiling.
    pub clip_depth: Option<i32>,
    /// Optional instance name.
    pub name: Option<String>,
    /// Visibility flag.
    pub visible: bool,
    /// Blend mode index.
    pub blend_mode: Option<u8>,
    /// Tween / morph ratio (0..=65535).
    pub ratio: u16,
}

impl Default for MovieClip {
    fn default() -> Self {
        Self::new(1)
    }
}

impl MovieClip {
    /// Creates a new movie clip with the specified total frames.
    pub fn new(total_frames: u32) -> Self {
        Self {
            current_frame: 1,
            total_frames: total_frames.max(1),
            play_state: PlayState::Play,
            frame_labels: HashMap::new(),
            slots: BTreeMap::new(),
            transform: SwfMatrix::default_matrix(),
            color_transform: SwfColorTransform::default_cxform(),
            clip_depth: None,
            name: None,
            visible: true,
            blend_mode: None,
            ratio: 0,
        }
    }

    /// Returns the current 1-based frame index.
    #[inline]
    pub fn clip_current_frame(&self) -> u32 {
        self.current_frame
    }

    /// Returns the total number of frames in this clip.
    #[inline]
    pub fn clip_total_frames(&self) -> u32 {
        self.total_frames
    }

    /// Returns the active playback state.
    #[inline]
    pub fn clip_play_state(&self) -> PlayState {
        self.play_state
    }

    /// Resumes timeline playback.
    #[inline]
    pub fn clip_play(&mut self) {
        self.play_state = PlayState::Play;
    }

    /// Halts timeline playback.
    #[inline]
    pub fn clip_stop(&mut self) {
        self.play_state = PlayState::Stop;
    }

    /// Advances playback to target and sets state to `Play`.
    pub fn clip_goto_and_play(&mut self, target: FrameTarget) -> Result<(), DisplayListError> {
        let frame = self.resolve_frame_target(&target)?;
        self.current_frame = frame;
        self.play_state = PlayState::Play;
        Ok(())
    }

    /// Advances playback to target and sets state to `Stop`.
    pub fn clip_goto_and_stop(&mut self, target: FrameTarget) -> Result<(), DisplayListError> {
        let frame = self.resolve_frame_target(&target)?;
        self.current_frame = frame;
        self.play_state = PlayState::Stop;
        Ok(())
    }

    /// Steps timeline to the next frame, wrapping to 1 if past the end.
    pub fn clip_next_frame(&mut self) {
        if self.current_frame >= self.total_frames {
            self.current_frame = 1;
        } else {
            self.current_frame += 1;
        }
    }

    /// Steps timeline to the previous frame, clamping at 1.
    pub fn clip_prev_frame(&mut self) {
        if self.current_frame > 1 {
            self.current_frame -= 1;
        } else {
            self.current_frame = self.total_frames;
        }
    }

    /// Advances timeline if currently playing.
    pub fn clip_step_timeline(&mut self) {
        if self.play_state == PlayState::Play && self.total_frames > 1 {
            self.clip_next_frame();
        }
    }

    /// Associates a string label with a 1-based frame index.
    pub fn clip_add_frame_label(
        &mut self,
        label: &str,
        frame: u32,
    ) -> Result<(), DisplayListError> {
        if frame == 0 || frame > self.total_frames {
            return Err(DisplayListError::InvalidFrame);
        }
        self.frame_labels.insert(label.to_string(), frame);
        Ok(())
    }

    /// Resolves a frame label string to its 1-based numeric index.
    #[inline]
    pub fn clip_frame_by_label(&self, label: &str) -> Option<u32> {
        self.frame_labels.get(label).copied()
    }

    fn resolve_frame_target(&self, target: &FrameTarget) -> Result<u32, DisplayListError> {
        match target {
            FrameTarget::Index(idx) => {
                if *idx == 0 || *idx > self.total_frames {
                    Err(DisplayListError::InvalidFrame)
                } else {
                    Ok(*idx)
                }
            }
            FrameTarget::Label(lbl) => self
                .clip_frame_by_label(lbl)
                .ok_or(DisplayListError::InvalidFrame),
        }
    }

    /// Returns the number of occupied child depth slots.
    #[inline]
    pub fn clip_child_count(&self) -> usize {
        self.slots.len()
    }

    /// Returns whether a display slot exists at the given depth.
    #[inline]
    pub fn clip_has_child_at(&self, depth: i32) -> bool {
        self.slots.contains_key(&depth)
    }

    /// Returns a reference to the display slot at the given depth.
    #[inline]
    pub fn clip_slot_at(&self, depth: i32) -> Option<&DisplaySlot> {
        self.slots.get(&depth)
    }

    /// Returns a mutable reference to the display slot at the given depth.
    #[inline]
    pub fn clip_slot_at_mut(&mut self, depth: i32) -> Option<&mut DisplaySlot> {
        self.slots.get_mut(&depth)
    }

    /// Inserts or replaces a display slot at its designated depth layer.
    pub fn clip_insert_slot(&mut self, slot: DisplaySlot) -> Result<(), DisplayListError> {
        if !(MIN_DEPTH..=MAX_DEPTH).contains(&slot.depth) {
            return Err(DisplayListError::InvalidDepth);
        }
        if let Some(cd) = slot.clip_depth {
            if cd <= slot.depth {
                return Err(DisplayListError::InvalidClipDepth);
            }
        }
        self.slots.insert(slot.depth, slot);
        Ok(())
    }

    /// Removes and returns the display slot at the specified depth layer.
    pub fn clip_remove_slot(&mut self, depth: i32) -> Option<DisplaySlot> {
        self.slots.remove(&depth)
    }

    /// Returns all occupied display slots in ascending depth order.
    pub fn clip_slots_depth_order(&self) -> Vec<&DisplaySlot> {
        self.slots.values().collect()
    }

    /// Returns local affine transformation matrix.
    #[inline]
    pub fn clip_local_transform(&self) -> &SwfMatrix {
        &self.transform
    }

    /// Updates local affine transformation matrix.
    #[inline]
    pub fn clip_set_local_transform(&mut self, matrix: SwfMatrix) {
        self.transform = matrix;
    }

    /// Returns local color multiplier and addition transform.
    #[inline]
    pub fn clip_local_color_transform(&self) -> &SwfColorTransform {
        &self.color_transform
    }

    /// Updates local color multiplier and addition transform.
    #[inline]
    pub fn clip_set_local_color_transform(&mut self, cxform: SwfColorTransform) {
        self.color_transform = cxform;
    }

    /// Returns stencil clipping depth ceiling if set.
    #[inline]
    pub fn clip_stencil_depth(&self) -> Option<i32> {
        self.clip_depth
    }

    /// Sets or clears stencil clipping depth ceiling.
    #[inline]
    pub fn clip_set_stencil_depth(&mut self, clip_depth: Option<i32>) {
        self.clip_depth = clip_depth;
    }

    /// Returns instance name string if assigned.
    #[inline]
    pub fn clip_name_string(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Sets instance name string.
    #[inline]
    pub fn clip_set_name_string(&mut self, name: Option<String>) {
        self.name = name;
    }

    /// Returns visibility flag.
    #[inline]
    pub fn clip_is_visible(&self) -> bool {
        self.visible
    }

    /// Sets visibility flag.
    #[inline]
    pub fn clip_set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Returns blend mode index.
    #[inline]
    pub fn clip_blend_mode(&self) -> Option<u8> {
        self.blend_mode
    }

    /// Sets blend mode index.
    #[inline]
    pub fn clip_set_blend_mode(&mut self, mode: Option<u8>) {
        self.blend_mode = mode;
    }

    /// Returns current ratio value.
    #[inline]
    pub fn clip_ratio(&self) -> u16 {
        self.ratio
    }

    /// Sets ratio value.
    #[inline]
    pub fn clip_set_ratio(&mut self, ratio: u16) {
        self.ratio = ratio;
    }

    /// Applies a `PlaceObject` record to the display list at its specified depth.
    pub fn clip_apply_place_record(
        &mut self,
        record: &PlaceObjectRecord,
    ) -> Result<(), DisplayListError> {
        if !(MIN_DEPTH..=MAX_DEPTH).contains(&record.depth) {
            return Err(DisplayListError::InvalidDepth);
        }
        if let Some(cd) = record.clip_depth {
            if cd <= record.depth {
                return Err(DisplayListError::InvalidClipDepth);
            }
        }

        if record.is_move {
            let slot = self
                .slots
                .get_mut(&record.depth)
                .ok_or(DisplayListError::SlotNotFound)?;

            if let Some(m) = record.matrix {
                slot.transform = m;
            }
            if let Some(cx) = record.color_transform {
                slot.color_transform = cx;
            }
            if let Some(r) = record.ratio {
                slot.ratio = Some(r);
                if let DisplayObject::Morph(ref mut morph) = slot.object {
                    morph.morph_set_ratio_val(r);
                }
            }
            if let Some(ref n) = record.name {
                slot.name = Some(n.clone());
            }
            if let Some(cd) = record.clip_depth {
                slot.clip_depth = Some(cd);
            }
            if let Some(bm) = record.blend_mode {
                slot.blend_mode = Some(bm);
            }
            if let Some(vis) = record.visible {
                slot.visible = vis;
            }
        } else {
            // Instantiate new slot or replace
            let mut slot = DisplaySlot::new(record.depth, DisplayObject::Clip(MovieClip::new(1)));
            slot.character_id = record.character_id;
            if let Some(m) = record.matrix {
                slot.transform = m;
            }
            if let Some(cx) = record.color_transform {
                slot.color_transform = cx;
            }
            if let Some(r) = record.ratio {
                slot.ratio = Some(r);
            }
            if let Some(ref n) = record.name {
                slot.name = Some(n.clone());
            }
            if let Some(cd) = record.clip_depth {
                slot.clip_depth = Some(cd);
            }
            if let Some(bm) = record.blend_mode {
                slot.blend_mode = Some(bm);
            }
            if let Some(vis) = record.visible {
                slot.visible = vis;
            }
            self.slots.insert(record.depth, slot);
        }

        Ok(())
    }

    /// Applies a `RemoveObject` record to the display list.
    pub fn clip_apply_remove_record(
        &mut self,
        record: &RemoveObjectRecord,
    ) -> Result<Option<DisplaySlot>, DisplayListError> {
        if !(MIN_DEPTH..=MAX_DEPTH).contains(&record.depth) {
            return Err(DisplayListError::InvalidDepth);
        }
        if let Some(expected_id) = record.character_id {
            if let Some(slot) = self.slots.get(&record.depth) {
                if slot.character_id != Some(expected_id) {
                    return Ok(None);
                }
            }
        }
        Ok(self.slots.remove(&record.depth))
    }

    /// Recursively steps timeline and all active child movieclips up to `MAX_HIERARCHY_DEPTH`.
    pub fn clip_step_hierarchy(&mut self, hierarchy_depth: usize) -> Result<(), DisplayListError> {
        if hierarchy_depth >= MAX_HIERARCHY_DEPTH {
            return Err(DisplayListError::HierarchyDepthExceeded);
        }

        self.clip_step_timeline();

        for slot in self.slots.values_mut() {
            if let DisplayObject::Clip(ref mut child_clip) = slot.object {
                child_clip.clip_step_hierarchy(hierarchy_depth + 1)?;
            }
        }

        Ok(())
    }
}
