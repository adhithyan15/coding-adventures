//! Pluggable runtime contract for Paint VM backends.
//!
//! This crate is intentionally backend-neutral. Direct2D, GDI, Metal, Cairo,
//! Skia, Vulkan, OpenGL/Mesa, WGPU, and OpenCL-backed experiments can all
//! describe their capabilities and register behind the same selector.

use paint_instructions::{
    GradientKind, PaintInstruction, PaintLayer, PaintScene, PathCommand, PixelContainer,
};

pub const VERSION: &str = "0.1.0";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintBackendFamily {
    Direct2D,
    Gdi,
    Metal,
    Cairo,
    Skia,
    Vulkan,
    OpenGl,
    Wgpu,
    OpenCl,
    Mesa,
    CoreGraphics,
    Ascii,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintAcceleration {
    Cpu,
    Gpu,
    Hybrid,
    Compute,
    Software,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PaintBackendTier {
    Tier0Scaffold,
    Tier1Smoke,
    Tier2NativeScenes,
    Tier3FullParity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SupportLevel {
    Unsupported,
    Degraded,
    Supported,
}

impl SupportLevel {
    fn satisfies(self, allow_degraded: bool) -> bool {
        matches!(self, SupportLevel::Supported)
            || (allow_degraded && matches!(self, SupportLevel::Degraded))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintPlatformSupport {
    pub windows: bool,
    pub macos: bool,
    pub ios: bool,
    pub linux: bool,
    pub bsd: bool,
    pub android: bool,
    pub web: bool,
    pub headless: bool,
}

impl PaintPlatformSupport {
    pub const fn none() -> Self {
        Self {
            windows: false,
            macos: false,
            ios: false,
            linux: false,
            bsd: false,
            android: false,
            web: false,
            headless: false,
        }
    }

    pub const fn all_desktop() -> Self {
        Self {
            windows: true,
            macos: true,
            ios: false,
            linux: true,
            bsd: true,
            android: false,
            web: false,
            headless: true,
        }
    }

    pub const fn windows() -> Self {
        Self {
            windows: true,
            headless: true,
            ..Self::none()
        }
    }

    pub const fn apple() -> Self {
        Self {
            macos: true,
            ios: true,
            headless: true,
            ..Self::none()
        }
    }

    pub const fn unix_headless() -> Self {
        Self {
            linux: true,
            bsd: true,
            headless: true,
            ..Self::none()
        }
    }

    pub const fn gpu_portable() -> Self {
        Self {
            windows: true,
            macos: true,
            linux: true,
            bsd: true,
            android: true,
            web: true,
            headless: true,
            ..Self::none()
        }
    }

    pub fn supports_current_platform(self) -> bool {
        (cfg!(target_os = "windows") && self.windows)
            || (cfg!(target_os = "macos") && self.macos)
            || (cfg!(target_os = "ios") && self.ios)
            || (cfg!(target_os = "linux") && self.linux)
            || (cfg!(target_os = "android") && self.android)
            || (cfg!(target_family = "wasm") && self.web)
            || (cfg!(target_os = "freebsd") && self.bsd)
            || (cfg!(target_os = "openbsd") && self.bsd)
            || (cfg!(target_os = "netbsd") && self.bsd)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintBackendCapabilities {
    pub rect: SupportLevel,
    pub line: SupportLevel,
    pub ellipse: SupportLevel,
    pub path: SupportLevel,
    pub path_arc_to: SupportLevel,
    pub glyph_run: SupportLevel,
    pub text: SupportLevel,
    pub image: SupportLevel,
    pub clip: SupportLevel,
    pub group: SupportLevel,
    pub group_transform: SupportLevel,
    pub group_opacity: SupportLevel,
    pub layer: SupportLevel,
    pub layer_opacity: SupportLevel,
    pub layer_filters: SupportLevel,
    pub layer_blend_modes: SupportLevel,
    pub linear_gradient: SupportLevel,
    pub radial_gradient: SupportLevel,
    pub antialiasing: SupportLevel,
    pub offscreen_pixels: SupportLevel,
}

impl PaintBackendCapabilities {
    pub const fn none() -> Self {
        Self {
            rect: SupportLevel::Unsupported,
            line: SupportLevel::Unsupported,
            ellipse: SupportLevel::Unsupported,
            path: SupportLevel::Unsupported,
            path_arc_to: SupportLevel::Unsupported,
            glyph_run: SupportLevel::Unsupported,
            text: SupportLevel::Unsupported,
            image: SupportLevel::Unsupported,
            clip: SupportLevel::Unsupported,
            group: SupportLevel::Unsupported,
            group_transform: SupportLevel::Unsupported,
            group_opacity: SupportLevel::Unsupported,
            layer: SupportLevel::Unsupported,
            layer_opacity: SupportLevel::Unsupported,
            layer_filters: SupportLevel::Unsupported,
            layer_blend_modes: SupportLevel::Unsupported,
            linear_gradient: SupportLevel::Unsupported,
            radial_gradient: SupportLevel::Unsupported,
            antialiasing: SupportLevel::Unsupported,
            offscreen_pixels: SupportLevel::Unsupported,
        }
    }

    pub fn missing_for(
        self,
        requirements: &PaintSceneRequirements,
        options: PaintRenderOptions,
    ) -> Vec<PaintFeature> {
        let mut missing = Vec::new();
        let allow_degraded = options.allow_degraded;
        let exact_text = options.require_exact_text;
        let text_ok = |level: SupportLevel| {
            if exact_text {
                matches!(level, SupportLevel::Supported)
            } else {
                level.satisfies(allow_degraded)
            }
        };

        push_missing(
            &mut missing,
            requirements.uses_rect,
            self.rect.satisfies(allow_degraded),
            PaintFeature::Rect,
        );
        push_missing(
            &mut missing,
            requirements.uses_line,
            self.line.satisfies(allow_degraded),
            PaintFeature::Line,
        );
        push_missing(
            &mut missing,
            requirements.uses_ellipse,
            self.ellipse.satisfies(allow_degraded),
            PaintFeature::Ellipse,
        );
        push_missing(
            &mut missing,
            requirements.uses_path,
            self.path.satisfies(allow_degraded),
            PaintFeature::Path,
        );
        push_missing(
            &mut missing,
            requirements.uses_path_arc_to,
            self.path_arc_to.satisfies(allow_degraded),
            PaintFeature::PathArcTo,
        );
        push_missing(
            &mut missing,
            requirements.uses_glyph_run,
            text_ok(self.glyph_run),
            PaintFeature::GlyphRun,
        );
        push_missing(
            &mut missing,
            requirements.uses_text,
            text_ok(self.text),
            PaintFeature::Text,
        );
        push_missing(
            &mut missing,
            requirements.uses_image,
            self.image.satisfies(allow_degraded),
            PaintFeature::Image,
        );
        push_missing(
            &mut missing,
            requirements.uses_clip,
            self.clip.satisfies(allow_degraded),
            PaintFeature::Clip,
        );
        push_missing(
            &mut missing,
            requirements.uses_group,
            self.group.satisfies(allow_degraded),
            PaintFeature::Group,
        );
        push_missing(
            &mut missing,
            requirements.uses_group_transform,
            self.group_transform.satisfies(allow_degraded),
            PaintFeature::GroupTransform,
        );
        push_missing(
            &mut missing,
            requirements.uses_group_opacity,
            self.group_opacity.satisfies(allow_degraded),
            PaintFeature::GroupOpacity,
        );
        push_missing(
            &mut missing,
            requirements.uses_layer,
            self.layer.satisfies(allow_degraded),
            PaintFeature::Layer,
        );
        push_missing(
            &mut missing,
            requirements.uses_layer_opacity,
            self.layer_opacity.satisfies(allow_degraded),
            PaintFeature::LayerOpacity,
        );
        push_missing(
            &mut missing,
            requirements.uses_layer_filters,
            self.layer_filters.satisfies(allow_degraded),
            PaintFeature::LayerFilters,
        );
        push_missing(
            &mut missing,
            requirements.uses_layer_blend_modes,
            self.layer_blend_modes.satisfies(allow_degraded),
            PaintFeature::LayerBlendModes,
        );
        push_missing(
            &mut missing,
            requirements.uses_linear_gradient,
            self.linear_gradient.satisfies(allow_degraded),
            PaintFeature::LinearGradient,
        );
        push_missing(
            &mut missing,
            requirements.uses_radial_gradient,
            self.radial_gradient.satisfies(allow_degraded),
            PaintFeature::RadialGradient,
        );
        push_missing(
            &mut missing,
            options.require_antialiasing,
            self.antialiasing.satisfies(allow_degraded),
            PaintFeature::Antialiasing,
        );
        push_missing(
            &mut missing,
            true,
            self.offscreen_pixels.satisfies(allow_degraded),
            PaintFeature::OffscreenPixels,
        );

        missing
    }
}

fn push_missing(
    missing: &mut Vec<PaintFeature>,
    required: bool,
    supported: bool,
    feature: PaintFeature,
) {
    if required && !supported {
        missing.push(feature);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintFeature {
    Rect,
    Line,
    Ellipse,
    Path,
    PathArcTo,
    GlyphRun,
    Text,
    Image,
    Clip,
    Group,
    GroupTransform,
    GroupOpacity,
    Layer,
    LayerOpacity,
    LayerFilters,
    LayerBlendModes,
    LinearGradient,
    RadialGradient,
    Antialiasing,
    OffscreenPixels,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintBackendDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub family: PaintBackendFamily,
    pub acceleration: PaintAcceleration,
    pub tier: PaintBackendTier,
    pub platforms: PaintPlatformSupport,
    pub capabilities: PaintBackendCapabilities,
    /// Lower values win within otherwise equivalent candidates.
    pub priority: u16,
}

impl PaintBackendDescriptor {
    pub fn scaffold(
        id: &'static str,
        display_name: &'static str,
        family: PaintBackendFamily,
        acceleration: PaintAcceleration,
        platforms: PaintPlatformSupport,
        priority: u16,
    ) -> Self {
        Self {
            id,
            display_name,
            family,
            acceleration,
            tier: PaintBackendTier::Tier0Scaffold,
            platforms,
            capabilities: PaintBackendCapabilities::none(),
            priority,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PaintSceneRequirements {
    pub uses_rect: bool,
    pub uses_line: bool,
    pub uses_ellipse: bool,
    pub uses_path: bool,
    pub uses_path_arc_to: bool,
    pub uses_glyph_run: bool,
    pub uses_text: bool,
    pub uses_image: bool,
    pub uses_clip: bool,
    pub uses_group: bool,
    pub uses_group_transform: bool,
    pub uses_group_opacity: bool,
    pub uses_layer: bool,
    pub uses_layer_opacity: bool,
    pub uses_layer_filters: bool,
    pub uses_layer_blend_modes: bool,
    pub uses_linear_gradient: bool,
    pub uses_radial_gradient: bool,
}

pub fn analyze_scene(scene: &PaintScene) -> PaintSceneRequirements {
    let mut requirements = PaintSceneRequirements::default();
    for instruction in &scene.instructions {
        analyze_instruction(instruction, &mut requirements);
    }
    requirements
}

fn analyze_instruction(instruction: &PaintInstruction, requirements: &mut PaintSceneRequirements) {
    match instruction {
        PaintInstruction::Rect(_) => requirements.uses_rect = true,
        PaintInstruction::Line(_) => requirements.uses_line = true,
        PaintInstruction::Ellipse(_) => requirements.uses_ellipse = true,
        PaintInstruction::Path(path) => {
            requirements.uses_path = true;
            requirements.uses_path_arc_to |= path
                .commands
                .iter()
                .any(|command| matches!(command, PathCommand::ArcTo { .. }));
        }
        PaintInstruction::Text(_) => requirements.uses_text = true,
        PaintInstruction::GlyphRun(_) => requirements.uses_glyph_run = true,
        PaintInstruction::Image(_) => requirements.uses_image = true,
        PaintInstruction::Gradient(gradient) => match gradient.kind {
            GradientKind::Linear { .. } => requirements.uses_linear_gradient = true,
            GradientKind::Radial { .. } => requirements.uses_radial_gradient = true,
        },
        PaintInstruction::Clip(clip) => {
            requirements.uses_clip = true;
            for child in &clip.children {
                analyze_instruction(child, requirements);
            }
        }
        PaintInstruction::Group(group) => {
            requirements.uses_group = true;
            requirements.uses_group_transform |= group.transform.is_some();
            requirements.uses_group_opacity |= opacity_is_effective(group.opacity);
            for child in &group.children {
                analyze_instruction(child, requirements);
            }
        }
        PaintInstruction::Layer(layer) => {
            analyze_layer(layer, requirements);
        }
    }
}

fn analyze_layer(layer: &PaintLayer, requirements: &mut PaintSceneRequirements) {
    requirements.uses_layer = true;
    requirements.uses_layer_opacity |= opacity_is_effective(layer.opacity);
    requirements.uses_group_transform |= layer.transform.is_some();
    requirements.uses_layer_filters |= layer
        .filters
        .as_ref()
        .is_some_and(|filters| !filters.is_empty());
    requirements.uses_layer_blend_modes |= layer.blend_mode.is_some();
    for child in &layer.children {
        analyze_instruction(child, requirements);
    }
}

fn opacity_is_effective(opacity: Option<f64>) -> bool {
    opacity.is_some_and(|value| (value - 1.0).abs() > f64::EPSILON)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaintBackendPreference {
    Auto,
    Named(String),
    PreferGpu,
    PreferCpu,
    PreferDeterministic,
    PreferNativePlatform,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaintRenderOptions {
    pub preference: PaintBackendPreference,
    pub allow_degraded: bool,
    pub require_antialiasing: bool,
    pub require_exact_text: bool,
}

impl Default for PaintRenderOptions {
    fn default() -> Self {
        Self {
            preference: PaintBackendPreference::Auto,
            allow_degraded: false,
            require_antialiasing: false,
            require_exact_text: true,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PaintRenderError {
    NoBackendsRegistered,
    NoCompatibleBackend {
        requirements: PaintSceneRequirements,
        missing: Vec<PaintFeature>,
    },
    BackendUnavailable {
        backend: &'static str,
        reason: &'static str,
    },
    RenderFailed {
        backend: &'static str,
        message: String,
    },
}

pub trait PaintRenderer {
    fn descriptor(&self) -> PaintBackendDescriptor;
    fn render(&self, scene: &PaintScene) -> Result<PixelContainer, PaintRenderError>;
}

#[derive(Default)]
pub struct PaintBackendRegistry<'a> {
    backends: Vec<&'a dyn PaintRenderer>,
}

impl<'a> PaintBackendRegistry<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, backend: &'a dyn PaintRenderer) {
        self.backends.push(backend);
    }

    pub fn select(
        &self,
        scene: &PaintScene,
        options: PaintRenderOptions,
    ) -> Result<&'a dyn PaintRenderer, PaintRenderError> {
        select_backend(scene, options, &self.backends)
    }

    pub fn render_auto(
        &self,
        scene: &PaintScene,
        options: PaintRenderOptions,
    ) -> Result<PixelContainer, PaintRenderError> {
        let backend = self.select(scene, options)?;
        backend.render(scene)
    }
}

pub fn select_backend<'a>(
    scene: &PaintScene,
    options: PaintRenderOptions,
    backends: &[&'a dyn PaintRenderer],
) -> Result<&'a dyn PaintRenderer, PaintRenderError> {
    if backends.is_empty() {
        return Err(PaintRenderError::NoBackendsRegistered);
    }

    let requirements = analyze_scene(scene);
    let mut best: Option<(&dyn PaintRenderer, i32)> = None;
    let mut missing_union = Vec::new();

    for backend in backends {
        let descriptor = backend.descriptor();
        if !matches_named_preference(descriptor, &options.preference) {
            continue;
        }
        if !descriptor.platforms.supports_current_platform() {
            continue;
        }
        let missing = descriptor
            .capabilities
            .missing_for(&requirements, options.clone());
        if !missing.is_empty() {
            extend_unique(&mut missing_union, missing);
            continue;
        }
        let rank = rank_descriptor(descriptor, &options.preference);
        if best.is_none_or(|(_, best_rank)| rank < best_rank) {
            best = Some((*backend, rank));
        }
    }

    best.map(|(backend, _)| backend)
        .ok_or(PaintRenderError::NoCompatibleBackend {
            requirements,
            missing: missing_union,
        })
}

fn matches_named_preference(
    descriptor: PaintBackendDescriptor,
    preference: &PaintBackendPreference,
) -> bool {
    match preference {
        PaintBackendPreference::Named(name) => descriptor.id == name,
        _ => true,
    }
}

fn rank_descriptor(descriptor: PaintBackendDescriptor, preference: &PaintBackendPreference) -> i32 {
    let mut rank = descriptor.priority as i32;
    rank -= match descriptor.tier {
        PaintBackendTier::Tier0Scaffold => 0,
        PaintBackendTier::Tier1Smoke => 10,
        PaintBackendTier::Tier2NativeScenes => 20,
        PaintBackendTier::Tier3FullParity => 30,
    };
    match preference {
        PaintBackendPreference::PreferGpu => {
            if matches!(
                descriptor.acceleration,
                PaintAcceleration::Gpu | PaintAcceleration::Hybrid
            ) {
                rank -= 100;
            }
        }
        PaintBackendPreference::PreferCpu | PaintBackendPreference::PreferDeterministic => {
            if matches!(
                descriptor.acceleration,
                PaintAcceleration::Cpu | PaintAcceleration::Software
            ) {
                rank -= 100;
            }
        }
        PaintBackendPreference::PreferNativePlatform => {
            if descriptor.platforms.supports_current_platform() {
                rank -= 50;
            }
        }
        PaintBackendPreference::Auto | PaintBackendPreference::Named(_) => {}
    }
    rank
}

fn extend_unique(existing: &mut Vec<PaintFeature>, incoming: Vec<PaintFeature>) {
    for feature in incoming {
        if !existing.contains(&feature) {
            existing.push(feature);
        }
    }
}

pub fn render_auto(
    scene: &PaintScene,
    options: PaintRenderOptions,
    backends: &[&dyn PaintRenderer],
) -> Result<PixelContainer, PaintRenderError> {
    select_backend(scene, options, backends)?.render(scene)
}

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{
        FillRule, GradientKind, GradientStop, PaintBase, PaintGradient, PaintGroup,
        PaintInstruction, PaintPath, PaintRect, PathCommand,
    };

    struct FakeBackend {
        descriptor: PaintBackendDescriptor,
    }

    impl PaintRenderer for FakeBackend {
        fn descriptor(&self) -> PaintBackendDescriptor {
            self.descriptor
        }

        fn render(&self, scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
            Ok(PixelContainer::new(scene.width as u32, scene.height as u32))
        }
    }

    fn rect_capabilities() -> PaintBackendCapabilities {
        PaintBackendCapabilities {
            rect: SupportLevel::Supported,
            offscreen_pixels: SupportLevel::Supported,
            ..PaintBackendCapabilities::none()
        }
    }

    fn descriptor(
        id: &'static str,
        priority: u16,
        acceleration: PaintAcceleration,
        capabilities: PaintBackendCapabilities,
    ) -> PaintBackendDescriptor {
        PaintBackendDescriptor {
            id,
            display_name: id,
            family: PaintBackendFamily::Cairo,
            acceleration,
            tier: PaintBackendTier::Tier1Smoke,
            platforms: PaintPlatformSupport::all_desktop(),
            capabilities,
            priority,
        }
    }

    #[test]
    fn analyzes_nested_scene_requirements() {
        let mut scene = PaintScene::new(100.0, 100.0);
        scene.instructions.push(PaintInstruction::Group(PaintGroup {
            base: PaintBase::default(),
            children: vec![
                PaintInstruction::Rect(PaintRect::filled(0.0, 0.0, 10.0, 10.0, "#000")),
                PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo { x: 0.0, y: 0.0 },
                        PathCommand::ArcTo {
                            rx: 10.0,
                            ry: 10.0,
                            x_rotation: 0.0,
                            large_arc: false,
                            sweep: true,
                            x: 20.0,
                            y: 20.0,
                        },
                    ],
                    fill: Some("#000".to_string()),
                    stroke: None,
                    stroke_width: None,
                    fill_rule: Some(FillRule::NonZero),
                    stroke_cap: None,
                    stroke_join: None,
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }),
            ],
            transform: Some([1.0, 0.0, 0.0, 1.0, 4.0, 0.0]),
            opacity: Some(0.5),
        }));

        let requirements = analyze_scene(&scene);
        assert!(requirements.uses_rect);
        assert!(requirements.uses_path);
        assert!(requirements.uses_path_arc_to);
        assert!(requirements.uses_group_transform);
        assert!(requirements.uses_group_opacity);
    }

    #[test]
    fn rejects_backend_that_cannot_satisfy_scene() {
        let mut scene = PaintScene::new(100.0, 100.0);
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase::default(),
                kind: GradientKind::Linear {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 100.0,
                    y2: 0.0,
                },
                stops: vec![GradientStop {
                    offset: 0.0,
                    color: "#000".to_string(),
                }],
            }));
        let backend = FakeBackend {
            descriptor: descriptor("rect-only", 1, PaintAcceleration::Cpu, rect_capabilities()),
        };
        let backends: [&dyn PaintRenderer; 1] = [&backend];

        let err = match select_backend(&scene, PaintRenderOptions::default(), &backends) {
            Ok(_) => panic!("rect-only backend should not satisfy gradient scene"),
            Err(err) => err,
        };
        assert!(matches!(err, PaintRenderError::NoCompatibleBackend { .. }));
    }

    #[test]
    fn selects_lowest_rank_compatible_backend() {
        let mut scene = PaintScene::new(10.0, 10.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 10.0, 10.0, "#000",
            )));
        let slow = FakeBackend {
            descriptor: descriptor("slow", 50, PaintAcceleration::Cpu, rect_capabilities()),
        };
        let fast = FakeBackend {
            descriptor: descriptor("fast", 10, PaintAcceleration::Gpu, rect_capabilities()),
        };
        let backends: [&dyn PaintRenderer; 2] = [&slow, &fast];

        let selected = select_backend(&scene, PaintRenderOptions::default(), &backends).unwrap();
        assert_eq!(selected.descriptor().id, "fast");
    }

    #[test]
    fn render_auto_uses_selected_backend() {
        let mut scene = PaintScene::new(4.0, 3.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 4.0, 3.0, "#000",
            )));
        let backend = FakeBackend {
            descriptor: descriptor("rect", 1, PaintAcceleration::Cpu, rect_capabilities()),
        };
        let backends: [&dyn PaintRenderer; 1] = [&backend];

        let pixels = render_auto(&scene, PaintRenderOptions::default(), &backends).unwrap();
        assert_eq!((pixels.width, pixels.height), (4, 3));
    }
}
