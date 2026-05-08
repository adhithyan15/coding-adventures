//! Core browser-shell state for Venture.
//!
//! This crate is deliberately UI- and network-free. It holds the deterministic
//! behavior from BR01 that the Win32, AppKit, and future shell layers can share:
//! navigation history, scroll clamping, and link hit-testing.

/// A 2D point in content or viewport coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Axis-aligned rectangle used for link hit-testing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x + self.width.max(0.0)
            && point.y < self.y + self.height.max(0.0)
    }
}

/// Clickable region recorded during layout or paint translation.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkRegion {
    pub rect: Rect,
    pub url: String,
}

impl LinkRegion {
    pub fn new(rect: Rect, url: impl Into<String>) -> Self {
        Self {
            rect,
            url: url.into(),
        }
    }
}

/// Hit-test a viewport-space click against content-space link regions.
pub fn hit_test_link<'a>(
    regions: &'a [LinkRegion],
    viewport_point: Point,
    scroll_y: f64,
) -> Option<&'a LinkRegion> {
    let content_point = Point {
        x: viewport_point.x,
        y: viewport_point.y + scroll_y.max(0.0),
    };
    regions
        .iter()
        .find(|region| region.rect.contains(content_point))
}

/// Browser navigation history: back stack, current URL, and forward stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationState {
    home_url: String,
    current_url: String,
    back_stack: Vec<String>,
    forward_stack: Vec<String>,
}

impl NavigationState {
    pub fn new(home_url: impl Into<String>) -> Self {
        let home_url = home_url.into();
        Self {
            current_url: home_url.clone(),
            home_url,
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
        }
    }

    pub fn current_url(&self) -> &str {
        &self.current_url
    }

    pub fn home_url(&self) -> &str {
        &self.home_url
    }

    pub fn back_stack(&self) -> &[String] {
        &self.back_stack
    }

    pub fn forward_stack(&self) -> &[String] {
        &self.forward_stack
    }

    pub fn can_go_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward_stack.is_empty()
    }

    pub fn navigate(&mut self, url: impl Into<String>) {
        let url = url.into();
        if url == self.current_url {
            return;
        }
        self.back_stack
            .push(std::mem::replace(&mut self.current_url, url));
        self.forward_stack.clear();
    }

    pub fn back(&mut self) -> Option<&str> {
        let previous = self.back_stack.pop()?;
        self.forward_stack
            .push(std::mem::replace(&mut self.current_url, previous));
        Some(&self.current_url)
    }

    pub fn forward(&mut self) -> Option<&str> {
        let next = self.forward_stack.pop()?;
        self.back_stack
            .push(std::mem::replace(&mut self.current_url, next));
        Some(&self.current_url)
    }

    pub fn home(&mut self) {
        self.navigate(self.home_url.clone());
    }

    pub fn reload(&self) -> &str {
        &self.current_url
    }
}

/// Scroll offset constrained by viewport and content height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollState {
    viewport_height: f64,
    content_height: f64,
    scroll_y: f64,
}

impl ScrollState {
    pub fn new(viewport_height: f64, content_height: f64) -> Self {
        let mut state = Self {
            viewport_height: viewport_height.max(0.0),
            content_height: content_height.max(0.0),
            scroll_y: 0.0,
        };
        state.clamp();
        state
    }

    pub fn scroll_y(&self) -> f64 {
        self.scroll_y
    }

    pub fn viewport_height(&self) -> f64 {
        self.viewport_height
    }

    pub fn content_height(&self) -> f64 {
        self.content_height
    }

    pub fn max_scroll_y(&self) -> f64 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    pub fn set_viewport_height(&mut self, height: f64) {
        self.viewport_height = height.max(0.0);
        self.clamp();
    }

    pub fn set_content_height(&mut self, height: f64) {
        self.content_height = height.max(0.0);
        self.clamp();
    }

    pub fn set_scroll_y(&mut self, scroll_y: f64) {
        self.scroll_y = scroll_y;
        self.clamp();
    }

    pub fn scroll_by(&mut self, delta_y: f64) {
        self.scroll_y += delta_y;
        self.clamp();
    }

    fn clamp(&mut self) {
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_y());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_tracks_back_forward_and_home() {
        let mut nav = NavigationState::new("http://info.cern.ch/");

        nav.navigate("http://example.com/a");
        nav.navigate("http://example.com/b");
        nav.navigate("http://example.com/c");
        assert_eq!(nav.current_url(), "http://example.com/c");
        assert_eq!(nav.back().unwrap(), "http://example.com/b");
        assert_eq!(nav.forward().unwrap(), "http://example.com/c");

        nav.back();
        nav.navigate("http://example.com/d");
        assert_eq!(nav.current_url(), "http://example.com/d");
        assert!(nav.forward_stack().is_empty());

        nav.home();
        assert_eq!(nav.current_url(), "http://info.cern.ch/");
        assert_eq!(nav.reload(), "http://info.cern.ch/");
    }

    #[test]
    fn navigation_noops_when_revisiting_current_url() {
        let mut nav = NavigationState::new("http://info.cern.ch/");

        nav.navigate("http://info.cern.ch/");

        assert!(nav.back_stack().is_empty());
        assert!(nav.forward_stack().is_empty());
        assert_eq!(nav.current_url(), "http://info.cern.ch/");
    }

    #[test]
    fn scroll_state_clamps_to_content_bounds() {
        let mut scroll = ScrollState::new(100.0, 250.0);

        scroll.scroll_by(75.0);
        assert_eq!(scroll.scroll_y(), 75.0);
        scroll.scroll_by(100.0);
        assert_eq!(scroll.scroll_y(), 150.0);
        scroll.scroll_by(-500.0);
        assert_eq!(scroll.scroll_y(), 0.0);

        scroll.set_scroll_y(120.0);
        scroll.set_content_height(80.0);
        assert_eq!(scroll.scroll_y(), 0.0);
        assert_eq!(scroll.max_scroll_y(), 0.0);
    }

    #[test]
    fn hit_testing_accounts_for_scroll_offset() {
        let regions = vec![
            LinkRegion::new(Rect::new(10.0, 20.0, 40.0, 15.0), "top"),
            LinkRegion::new(Rect::new(10.0, 120.0, 40.0, 15.0), "below"),
        ];

        assert_eq!(
            hit_test_link(&regions, Point::new(20.0, 25.0), 0.0)
                .unwrap()
                .url,
            "top"
        );
        assert_eq!(
            hit_test_link(&regions, Point::new(20.0, 25.0), 100.0)
                .unwrap()
                .url,
            "below"
        );
        assert!(hit_test_link(&regions, Point::new(0.0, 0.0), 0.0).is_none());
    }
}
