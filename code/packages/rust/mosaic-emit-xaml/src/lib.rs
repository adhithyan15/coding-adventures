//! # mosaic-emit-xaml — WinUI 3 / XAML backend for the Mosaic pipeline.
//!
//! This crate is the WinUI 3 / XAML peer of `mosaic-emit-react`,
//! `mosaic-emit-swiftui`, `mosaic-emit-qt`, `mosaic-emit-webcomponent`, and
//! `mosaic-emit-html`. It consumes the three-file pipeline output produced
//! by [`mosmodel-compiler`], [`moslayout-compiler`], and
//! [`mosstyle-compiler`] and produces a *triple* of generated files for one
//! WinUI 3 UserControl:
//!
//! - **`{Component}.xaml`** — the XAML markup, with a `<UserControl>` root
//!   containing the lowered moslayout tree.
//! - **`{Component}.xaml.cs`** — the C# code-behind with one
//!   `DependencyProperty` per `.mil` slot, the UI24 `Dispatch` event, and
//!   the `InitializeComponent()` boilerplate.
//! - **`{Component}.Event.cs`** — the discriminated event-union as C#
//!   records (matches UI24 §3.1 shape).
//!
//! See `code/specs/mosaic-emit-xaml.md` for the design.
//!
//! ## Output triple — why three files
//!
//! Unlike the React backend (which emits a single `.tsx` file) or the
//! SwiftUI backend (a single `.swift` file), WinUI 3 fundamentally requires
//! the markup and code-behind to be separated. A `<UserControl>` element
//! cannot live in C# alongside its class definition: the XAML compiler
//! parses `.xaml` files at build time and generates a partial class that
//! matches the code-behind's `partial class`. Splitting the event union
//! into its own file is a Mosaic convention — it keeps the `.xaml.cs` from
//! ballooning when a component has many emits, and it lets host
//! applications import just the event types without pulling in the
//! UserControl.
//!
//! ## PR-1 scope
//!
//! This first PR implements the **scaffold** plus the **nine simple kernel
//! primitives** from UI29 §2.1:
//!
//! | UI29 primitive | XAML lowering                                  |
//! |----------------|------------------------------------------------|
//! | `Box`          | `<Border>` (or `<ContentPresenter>` when no padding/background) |
//! | `Row`          | `<StackPanel Orientation="Horizontal">`        |
//! | `Column`       | `<StackPanel Orientation="Vertical">`          |
//! | `Stack`        | `<Grid>` (single cell, z-axis stacking)        |
//! | `Text`         | `<TextBlock>` with literal or `{x:Bind}` text  |
//! | `Image`        | `<Image Source="..."/>`                        |
//! | `Spacer`       | `<Rectangle/>` flex glue                       |
//! | `Divider`      | `<Border BorderThickness="..." />`             |
//! | `Icon`         | `<FontIcon Glyph="..."/>`                      |
//! | `HostSurface`  | Styled `<Border>` + node-bound `<ContentPresenter>` |
//!
//! Plus the UI24 dispatch contract (one `Dispatch` event per UserControl)
//! and the slot → `DependencyProperty` translation table.
//!
//! ## What is NOT in this first pass (deferred per spec §17)
//!
//! - **PR-2** — `If`, `Else`, `For`, and the `ExprLowerer`. These currently
//!   surface as `PipelineEmitError::UnsupportedPrimitive` /
//!   `UnsupportedExpression` so authors get a clear "not yet supported"
//!   diagnostic.
//! - **PR-3** — `HostInput`, `HostButton`, `HostScroll`.
//! - **PR-4** — `HostTable` plus its four section sub-tags
//!   (`HostTableColGroup`, `HostTableHead`, `HostTableBody`,
//!   `HostTableFoot`).
//! - **PR-5** — component-reference resolver (`<pkg:ComponentName/>`) +
//!   `--package-mode` CLI flag.
//! - **PR-6** — `mosaic-pkg-grid` compiled through this backend + a
//!   VisiCalc Windows demo proving the architecture end-to-end.
//!
//! ## Public API
//!
//! ```ignore
//! use mosaic_emit_xaml::{from_pipeline, EmitOptions, XamlEmitResult};
//!
//! let result: XamlEmitResult = from_pipeline(
//!     &interface,   // MosmodelComponent (.mil)
//!     &layout,      // LayoutDef        (.mll)
//!     &style,       // StyleDef         (.msl)
//!     None,         // optional package manifest (PR-5)
//!     &EmitOptions::default(),
//! )?;
//! // result.xaml         → write to MyComponent.xaml
//! // result.code_behind  → write to MyComponent.xaml.cs
//! // result.events       → write to MyComponent.Event.cs
//! ```

pub mod pipeline;

pub use pipeline::{
    from_pipeline, ComponentRef, ComponentRegistry, EmitOptions, EmittedFile, PipelineEmitError,
    ProjectFiles, XamlEmitResult,
};

/// Crate version string. Kept in sync with `Cargo.toml`'s `[package]`
/// `version` field by convention; do not edit by hand.
pub const VERSION: &str = "0.1.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_0_1_0() {
        assert_eq!(VERSION, "0.1.0");
    }
}
