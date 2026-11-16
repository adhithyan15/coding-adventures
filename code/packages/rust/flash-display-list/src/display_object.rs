//! Flash display-object tree hierarchy and slot containers.

use swf_container::{SwfColorTransform, SwfMatrix};
use swf_shape_to_paint::ParsedShape;

use crate::button::SimpleButton;
use crate::morph::MorphShape;
use crate::movie_clip::MovieClip;
use crate::text::TextField;

/// A node in the Flash display object hierarchy.
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayObject {
    /// Self-contained movie clip with nested timeline and child display list.
    Clip(MovieClip),
    /// Interactive button control.
    Button(SimpleButton),
    /// Dynamic or input text field.
    Text(TextField),
    /// Tweened vector morph shape.
    Morph(MorphShape),
    /// Static vector shape definition.
    Shape(ParsedShape),
}

impl DisplayObject {
    /// Returns a reference to the inner MovieClip, if this is a Clip.
    #[inline]
    pub fn obj_as_clip(&self) -> Option<&MovieClip> {
        match self {
            Self::Clip(c) => Some(c),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner MovieClip, if this is a Clip.
    #[inline]
    pub fn obj_as_clip_mut(&mut self) -> Option<&mut MovieClip> {
        match self {
            Self::Clip(c) => Some(c),
            _ => None,
        }
    }

    /// Returns a reference to the inner SimpleButton, if this is a Button.
    #[inline]
    pub fn obj_as_button(&self) -> Option<&SimpleButton> {
        match self {
            Self::Button(b) => Some(b),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner SimpleButton, if this is a Button.
    #[inline]
    pub fn obj_as_button_mut(&mut self) -> Option<&mut SimpleButton> {
        match self {
            Self::Button(b) => Some(b),
            _ => None,
        }
    }

    /// Returns a reference to the inner TextField, if this is a Text.
    #[inline]
    pub fn obj_as_text(&self) -> Option<&TextField> {
        match self {
            Self::Text(t) => Some(t),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner TextField, if this is a Text.
    #[inline]
    pub fn obj_as_text_mut(&mut self) -> Option<&mut TextField> {
        match self {
            Self::Text(t) => Some(t),
            _ => None,
        }
    }

    /// Returns a reference to the inner MorphShape, if this is a Morph.
    #[inline]
    pub fn obj_as_morph(&self) -> Option<&MorphShape> {
        match self {
            Self::Morph(m) => Some(m),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner MorphShape, if this is a Morph.
    #[inline]
    pub fn obj_as_morph_mut(&mut self) -> Option<&mut MorphShape> {
        match self {
            Self::Morph(m) => Some(m),
            _ => None,
        }
    }

    /// Returns a reference to the inner ParsedShape, if this is a Shape.
    #[inline]
    pub fn obj_as_shape(&self) -> Option<&ParsedShape> {
        match self {
            Self::Shape(s) => Some(s),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner ParsedShape, if this is a Shape.
    #[inline]
    pub fn obj_as_shape_mut(&mut self) -> Option<&mut ParsedShape> {
        match self {
            Self::Shape(s) => Some(s),
            _ => None,
        }
    }

    /// Computes the unscaled local bounding box `(min_x, min_y, max_x, max_y)` in pixels.
    pub fn obj_bounds_pixels(&self) -> (f64, f64, f64, f64) {
        match self {
            Self::Shape(s) => (
                s.bounds.xmin as f64 / 20.0,
                s.bounds.ymin as f64 / 20.0,
                s.bounds.xmax as f64 / 20.0,
                s.bounds.ymax as f64 / 20.0,
            ),
            Self::Morph(m) => (
                m.start_shape.bounds.xmin as f64 / 20.0,
                m.start_shape.bounds.ymin as f64 / 20.0,
                m.start_shape.bounds.xmax as f64 / 20.0,
                m.start_shape.bounds.ymax as f64 / 20.0,
            ),
            Self::Text(t) => (
                t.bounds.0,
                t.bounds.1,
                t.bounds.0 + t.bounds.2,
                t.bounds.1 + t.bounds.3,
            ),
            Self::Button(b) => {
                if let Some(active) = b.button_active_record() {
                    active.obj_bounds_pixels()
                } else if let Some(hit) = b.button_hit_record() {
                    hit.obj_bounds_pixels()
                } else {
                    (0.0, 0.0, 0.0, 0.0)
                }
            }
            Self::Clip(c) => {
                let slots = c.clip_slots_depth_order();
                if slots.is_empty() {
                    return (0.0, 0.0, 0.0, 0.0);
                }
                let mut min_x = f64::INFINITY;
                let mut min_y = f64::INFINITY;
                let mut max_x = f64::NEG_INFINITY;
                let mut max_y = f64::NEG_INFINITY;
                for slot in slots {
                    let b = slot.object.obj_bounds_pixels();
                    let tx = slot.transform.matrix_translate_pixels();
                    let (sx, sy) = slot.transform.matrix_scale();
                    let (x0, y0) = (b.0 * sx + tx.0, b.1 * sy + tx.1);
                    let (x1, y1) = (b.2 * sx + tx.0, b.3 * sy + tx.1);
                    min_x = min_x.min(x0.min(x1));
                    min_y = min_y.min(y0.min(y1));
                    max_x = max_x.max(x0.max(x1));
                    max_y = max_y.max(y0.max(y1));
                }
                if min_x.is_infinite() {
                    (0.0, 0.0, 0.0, 0.0)
                } else {
                    (min_x, min_y, max_x, max_y)
                }
            }
        }
    }
}

/// A depth-sorted display slot occupying a discrete layer on a MovieClip timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplaySlot {
    /// Depth layer index (`-16384..=65535`).
    pub depth: i32,
    /// Character ID in the SWF dictionary, if instanced from a definition.
    pub character_id: Option<u16>,
    /// The hosted display object.
    pub object: DisplayObject,
    /// 2D affine transformation matrix.
    pub transform: SwfMatrix,
    /// Color multiplier and addition transform.
    pub color_transform: SwfColorTransform,
    /// Optional stencil mask clipping target depth.
    pub clip_depth: Option<i32>,
    /// Optional tween/morph ratio (`0..=65535`).
    pub ratio: Option<u16>,
    /// Optional instance name for ActionScript pathing.
    pub name: Option<String>,
    /// Visibility flag.
    pub visible: bool,
    /// Flash blend mode index.
    pub blend_mode: Option<u8>,
}

impl DisplaySlot {
    /// Creates a new display slot with identity transforms.
    pub fn new(depth: i32, object: DisplayObject) -> Self {
        Self {
            depth,
            character_id: None,
            object,
            transform: SwfMatrix::default_matrix(),
            color_transform: SwfColorTransform::default_cxform(),
            clip_depth: None,
            ratio: None,
            name: None,
            visible: true,
            blend_mode: None,
        }
    }

    /// Returns the depth layer index.
    #[inline]
    pub fn slot_depth(&self) -> i32 {
        self.depth
    }

    /// Returns the character identifier if known.
    #[inline]
    pub fn slot_character_id(&self) -> Option<u16> {
        self.character_id
    }

    /// Returns a reference to the display object in this slot.
    #[inline]
    pub fn slot_object(&self) -> &DisplayObject {
        &self.object
    }

    /// Returns a mutable reference to the display object in this slot.
    #[inline]
    pub fn slot_object_mut(&mut self) -> &mut DisplayObject {
        &mut self.object
    }

    /// Returns the affine matrix transformation.
    #[inline]
    pub fn slot_transform(&self) -> &SwfMatrix {
        &self.transform
    }

    /// Updates the affine matrix transformation.
    #[inline]
    pub fn slot_set_transform(&mut self, matrix: SwfMatrix) {
        self.transform = matrix;
    }

    /// Returns the color transform.
    #[inline]
    pub fn slot_color_transform(&self) -> &SwfColorTransform {
        &self.color_transform
    }

    /// Updates the color transform.
    #[inline]
    pub fn slot_set_color_transform(&mut self, cxform: SwfColorTransform) {
        self.color_transform = cxform;
    }

    /// Returns the clipping mask depth limit, if active.
    #[inline]
    pub fn slot_clip_depth(&self) -> Option<i32> {
        self.clip_depth
    }

    /// Sets or clears the clipping mask depth limit.
    #[inline]
    pub fn slot_set_clip_depth(&mut self, clip_depth: Option<i32>) {
        self.clip_depth = clip_depth;
    }

    /// Returns the morph or tween ratio.
    #[inline]
    pub fn slot_ratio(&self) -> Option<u16> {
        self.ratio
    }

    /// Sets the morph or tween ratio.
    #[inline]
    pub fn slot_set_ratio(&mut self, ratio: Option<u16>) {
        self.ratio = ratio;
    }

    /// Returns the instance name if assigned.
    #[inline]
    pub fn slot_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Sets the instance name.
    #[inline]
    pub fn slot_set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    /// Returns whether this slot is visible.
    #[inline]
    pub fn slot_is_visible(&self) -> bool {
        self.visible
    }

    /// Sets slot visibility.
    #[inline]
    pub fn slot_set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Returns the blend mode if specified.
    #[inline]
    pub fn slot_blend_mode(&self) -> Option<u8> {
        self.blend_mode
    }

    /// Sets the blend mode.
    #[inline]
    pub fn slot_set_blend_mode(&mut self, mode: Option<u8>) {
        self.blend_mode = mode;
    }
}
