//! Root display canvas and scene coordinator (`Stage`).

use paint_instructions::PaintScene;
use paint_palette_blend::{diff_paint_scene_into, PatchOp, PatchSummary};
use point2d::Point;

use crate::display_object::{DisplayObject, DisplaySlot};
use crate::error::DisplayListError;
use crate::events::DisplayListEvent;
use crate::lowering::{lower_movie_clip_to_scene, swf_matrix_to_affine};
use crate::movie_clip::MovieClip;
use crate::tag_parse::{PlaceObjectRecord, RemoveObjectRecord};

/// The root canvas and event dispatcher for the interactive Flash playback environment.
#[derive(Debug, Clone, PartialEq)]
pub struct Stage {
    /// Width of the stage canvas in pixels.
    pub width_px: f64,
    /// Height of the stage canvas in pixels.
    pub height_px: f64,
    /// Background color CSS string (e.g. `"#ffffff"`).
    pub background: String,
    /// Playback frame rate in frames per second.
    pub frame_rate: f64,
    /// Horizontal mouse position in stage coordinates.
    pub mouse_x: f64,
    /// Vertical mouse position in stage coordinates.
    pub mouse_y: f64,
    /// Whether the primary mouse button is depressed.
    pub mouse_down: bool,
    /// Identifier of currently focused display character.
    pub focused_character: Option<u16>,
    /// Root movie clip containing stage-level display slots.
    pub root: MovieClip,
}

impl Stage {
    /// Creates a new Stage with specified dimensions and frame rate.
    pub fn new(width_px: f64, height_px: f64, frame_rate: f64) -> Self {
        Self {
            width_px: width_px.max(1.0),
            height_px: height_px.max(1.0),
            background: "#ffffff".to_string(),
            frame_rate: frame_rate.max(0.1),
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_down: false,
            focused_character: None,
            root: MovieClip::new(1),
        }
    }

    /// Returns the stage dimensions `(width, height)` in pixels.
    #[inline]
    pub fn stage_dimensions(&self) -> (f64, f64) {
        (self.width_px, self.height_px)
    }

    /// Updates the stage dimensions in pixels.
    #[inline]
    pub fn stage_set_dimensions(&mut self, width: f64, height: f64) {
        self.width_px = width.max(1.0);
        self.height_px = height.max(1.0);
    }

    /// Returns the stage background color string.
    #[inline]
    pub fn stage_background_color(&self) -> &str {
        &self.background
    }

    /// Updates the stage background color.
    #[inline]
    pub fn stage_set_background_color(&mut self, color: &str) {
        self.background = color.to_string();
    }

    /// Returns the playback frame rate.
    #[inline]
    pub fn stage_frame_rate(&self) -> f64 {
        self.frame_rate
    }

    /// Updates the playback frame rate.
    #[inline]
    pub fn stage_set_frame_rate(&mut self, fps: f64) {
        self.frame_rate = fps.max(0.1);
    }

    /// Returns the current mouse position `(x, y)`.
    #[inline]
    pub fn stage_mouse_position(&self) -> (f64, f64) {
        (self.mouse_x, self.mouse_y)
    }

    /// Returns whether the mouse button is pressed.
    #[inline]
    pub fn stage_is_mouse_down(&self) -> bool {
        self.mouse_down
    }

    /// Returns a reference to the root movie clip.
    #[inline]
    pub fn stage_root_clip(&self) -> &MovieClip {
        &self.root
    }

    /// Returns a mutable reference to the root movie clip.
    #[inline]
    pub fn stage_root_clip_mut(&mut self) -> &mut MovieClip {
        &mut self.root
    }

    /// Advances active timelines on the stage, emitting lifecycle events.
    pub fn stage_advance_frame(&mut self) -> Result<Vec<DisplayListEvent>, DisplayListError> {
        self.root.clip_step_hierarchy(0)?;
        Ok(vec![DisplayListEvent::EnterFrame])
    }

    /// Dispatches mouse motion across the stage and interactive controls.
    pub fn stage_handle_mouse_move(&mut self, x: f64, y: f64) -> Vec<DisplayListEvent> {
        self.mouse_x = x;
        self.mouse_y = y;

        let mut events = vec![DisplayListEvent::MouseMove { x, y }];

        for slot in self.root.slots.values_mut() {
            if let DisplayObject::Button(ref mut b) = slot.object {
                let affine = swf_matrix_to_affine(&slot.transform);
                let hit = if let Some(inv) = affine.invert() {
                    let local_pt = inv.apply_to_point(Point::new(x, y));
                    let bounds = if let Some(h) = b.button_hit_record() {
                        h.obj_bounds_pixels_bounded(0)
                            .unwrap_or((0.0, 0.0, 0.0, 0.0))
                    } else if let Some(active) = b.button_active_record() {
                        active
                            .obj_bounds_pixels_bounded(0)
                            .unwrap_or((0.0, 0.0, 0.0, 0.0))
                    } else {
                        (0.0, 0.0, 0.0, 0.0)
                    };
                    local_pt.x >= bounds.0
                        && local_pt.x <= bounds.2
                        && local_pt.y >= bounds.1
                        && local_pt.y <= bounds.3
                } else {
                    false
                };

                if let Some(btn_event) = b.button_process_mouse_move(hit) {
                    events.push(DisplayListEvent::ButtonAction {
                        depth: slot.depth,
                        character_id: slot.character_id,
                        event: btn_event,
                    });
                }
            }
        }

        events
    }

    /// Dispatches mouse button down events across interactive controls.
    pub fn stage_handle_mouse_down(&mut self) -> Vec<DisplayListEvent> {
        self.mouse_down = true;
        let mut events = vec![DisplayListEvent::MouseDown {
            x: self.mouse_x,
            y: self.mouse_y,
        }];

        for slot in self.root.slots.values_mut() {
            if let DisplayObject::Button(ref mut b) = slot.object {
                let affine = swf_matrix_to_affine(&slot.transform);
                let hit = if let Some(inv) = affine.invert() {
                    let local_pt = inv.apply_to_point(Point::new(self.mouse_x, self.mouse_y));
                    let bounds = if let Some(h) = b.button_hit_record() {
                        h.obj_bounds_pixels_bounded(0)
                            .unwrap_or((0.0, 0.0, 0.0, 0.0))
                    } else if let Some(active) = b.button_active_record() {
                        active
                            .obj_bounds_pixels_bounded(0)
                            .unwrap_or((0.0, 0.0, 0.0, 0.0))
                    } else {
                        (0.0, 0.0, 0.0, 0.0)
                    };
                    local_pt.x >= bounds.0
                        && local_pt.x <= bounds.2
                        && local_pt.y >= bounds.1
                        && local_pt.y <= bounds.3
                } else {
                    false
                };

                if let Some(btn_event) = b.button_process_mouse_down(hit) {
                    events.push(DisplayListEvent::ButtonAction {
                        depth: slot.depth,
                        character_id: slot.character_id,
                        event: btn_event,
                    });
                }
            }
        }

        events
    }

    /// Dispatches mouse button up events across interactive controls.
    pub fn stage_handle_mouse_up(&mut self) -> Vec<DisplayListEvent> {
        self.mouse_down = false;
        let mut events = vec![DisplayListEvent::MouseUp {
            x: self.mouse_x,
            y: self.mouse_y,
        }];

        for slot in self.root.slots.values_mut() {
            if let DisplayObject::Button(ref mut b) = slot.object {
                let affine = swf_matrix_to_affine(&slot.transform);
                let hit = if let Some(inv) = affine.invert() {
                    let local_pt = inv.apply_to_point(Point::new(self.mouse_x, self.mouse_y));
                    let bounds = if let Some(h) = b.button_hit_record() {
                        h.obj_bounds_pixels_bounded(0)
                            .unwrap_or((0.0, 0.0, 0.0, 0.0))
                    } else if let Some(active) = b.button_active_record() {
                        active
                            .obj_bounds_pixels_bounded(0)
                            .unwrap_or((0.0, 0.0, 0.0, 0.0))
                    } else {
                        (0.0, 0.0, 0.0, 0.0)
                    };
                    local_pt.x >= bounds.0
                        && local_pt.x <= bounds.2
                        && local_pt.y >= bounds.1
                        && local_pt.y <= bounds.3
                } else {
                    false
                };

                if let Some(btn_event) = b.button_process_mouse_up(hit) {
                    events.push(DisplayListEvent::ButtonAction {
                        depth: slot.depth,
                        character_id: slot.character_id,
                        event: btn_event,
                    });
                }
            }
        }

        events
    }

    /// Lowers the current stage display tree into a `PaintScene`.
    pub fn stage_lower_to_scene(&self) -> Result<PaintScene, DisplayListError> {
        lower_movie_clip_to_scene(&self.root, self.width_px, self.height_px, &self.background)
    }

    /// Computes retained-mode scene diffs against `prev`, writing patches into `out_ops`.
    pub fn stage_diff_scene_into(
        &self,
        prev: &PaintScene,
        out_ops: &mut [PatchOp],
    ) -> Result<PatchSummary, DisplayListError> {
        let current = self.stage_lower_to_scene()?;
        diff_paint_scene_into(prev, &current, out_ops).map_err(DisplayListError::from)
    }

    /// Applies a `PlaceObject` record to the root display list.
    #[inline]
    pub fn stage_apply_place_record(
        &mut self,
        record: &PlaceObjectRecord,
    ) -> Result<(), DisplayListError> {
        self.root.clip_apply_place_record(record)
    }

    /// Applies a `RemoveObject` record to the root display list.
    #[inline]
    pub fn stage_apply_remove_record(
        &mut self,
        record: &RemoveObjectRecord,
    ) -> Result<Option<DisplaySlot>, DisplayListError> {
        self.root.clip_apply_remove_record(record)
    }
}
