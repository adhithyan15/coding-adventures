//! Dynamic and input text display object with ActionScript variable binding (`TextField`).

use swf_shape_to_paint::ColorRgba;

/// A text rendering display object supporting dynamic text, input text,
/// and bidirectional ActionScript variable bindings.
#[derive(Debug, Clone, PartialEq)]
pub struct TextField {
    /// Bounding rectangle `(x, y, width, height)` in pixels.
    pub bounds: (f64, f64, f64, f64),
    /// Current text content.
    pub text: String,
    /// Bound variable name in ActionScript environment.
    pub variable_name: Option<String>,
    /// Text font size in points/pixels.
    pub font_size: f64,
    /// Font family or linkage name.
    pub font_name: Option<String>,
    /// Color of the rendered glyphs.
    pub text_color: ColorRgba,
    /// Whether this field accepts user keyboard input.
    pub is_input: bool,
    /// Whether this field allows multiple lines.
    pub is_multiline: bool,
    /// Whether this field has a rectangular border.
    pub has_border: bool,
    /// Color of the border outline.
    pub border_color: ColorRgba,
    /// Optional solid background fill color.
    pub background_fill: Option<ColorRgba>,
}

impl TextField {
    /// Creates a new text field with the given bounds and initial string.
    pub fn new(x: f64, y: f64, width: f64, height: f64, initial_text: &str) -> Self {
        Self {
            bounds: (x, y, width.max(1.0), height.max(1.0)),
            text: initial_text.to_string(),
            variable_name: None,
            font_size: 12.0,
            font_name: None,
            text_color: ColorRgba::rgba(0, 0, 0, 255),
            is_input: false,
            is_multiline: false,
            has_border: false,
            border_color: ColorRgba::rgba(0, 0, 0, 255),
            background_fill: None,
        }
    }

    /// Returns the current text content.
    #[inline]
    pub fn field_text_content(&self) -> &str {
        &self.text
    }

    /// Updates the text content.
    #[inline]
    pub fn field_set_text_content(&mut self, text: &str) {
        self.text = text.to_string();
    }

    /// Returns the ActionScript variable bound to this text field, if any.
    #[inline]
    pub fn field_bound_variable(&self) -> Option<&str> {
        self.variable_name.as_deref()
    }

    /// Sets or clears the bound ActionScript variable name.
    #[inline]
    pub fn field_set_bound_variable(&mut self, var_name: Option<String>) {
        self.variable_name = var_name;
    }

    /// Synchronizes text if the given variable name matches the bound variable.
    pub fn field_update_from_variable(&mut self, var_name: &str, value: &str) -> bool {
        if let Some(bound) = &self.variable_name {
            if bound == var_name {
                self.text = value.to_string();
                return true;
            }
        }
        false
    }

    /// Returns the bounding box `(x, y, width, height)`.
    #[inline]
    pub fn field_text_bounds(&self) -> (f64, f64, f64, f64) {
        self.bounds
    }

    /// Updates the text field bounds.
    #[inline]
    pub fn field_set_text_bounds(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.bounds = (x, y, width.max(1.0), height.max(1.0));
    }

    /// Returns the text color.
    #[inline]
    pub fn field_text_color(&self) -> ColorRgba {
        self.text_color
    }

    /// Sets the text color.
    #[inline]
    pub fn field_set_text_color(&mut self, color: ColorRgba) {
        self.text_color = color;
    }

    /// Returns the font size in points.
    #[inline]
    pub fn field_font_size_pt(&self) -> f64 {
        self.font_size
    }

    /// Sets the font size in points.
    #[inline]
    pub fn field_set_font_size_pt(&mut self, size: f64) {
        self.font_size = size.max(1.0);
    }

    /// Returns the font family name if specified.
    #[inline]
    pub fn field_font_name_str(&self) -> Option<&str> {
        self.font_name.as_deref()
    }

    /// Sets the font family name.
    #[inline]
    pub fn field_set_font_name_str(&mut self, font: Option<String>) {
        self.font_name = font;
    }

    /// Returns whether this text field allows user editing.
    #[inline]
    pub fn field_is_input_type(&self) -> bool {
        self.is_input
    }

    /// Sets whether this text field is an input field.
    #[inline]
    pub fn field_set_input_type(&mut self, is_input: bool) {
        self.is_input = is_input;
    }

    /// Returns whether multiline text is supported.
    #[inline]
    pub fn field_is_multiline(&self) -> bool {
        self.is_multiline
    }

    /// Sets whether multiline text is supported.
    #[inline]
    pub fn field_set_multiline(&mut self, multiline: bool) {
        self.is_multiline = multiline;
    }

    /// Returns whether the border outline is enabled.
    #[inline]
    pub fn field_has_border(&self) -> bool {
        self.has_border
    }

    /// Sets the border outline and its color.
    #[inline]
    pub fn field_set_border(&mut self, has_border: bool, color: ColorRgba) {
        self.has_border = has_border;
        self.border_color = color;
    }

    /// Returns the background fill color, if any.
    #[inline]
    pub fn field_background_fill(&self) -> Option<ColorRgba> {
        self.background_fill
    }

    /// Sets the optional background fill color.
    #[inline]
    pub fn field_set_background_fill(&mut self, color: Option<ColorRgba>) {
        self.background_fill = color;
    }
}
