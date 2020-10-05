use super::super::types::{Color, IndirectFontRef};

/// All purpose alignment type.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Alignment {
    Center,
    Left,
    Right,
}

/// Track render style.
#[derive(Debug, Clone)]
pub(crate) struct RenderStyle {
    pub(crate) font: IndirectFontRef,
    pub(crate) size: i32,
    pub(crate) foreground: Color,
    pub(crate) background: Color,
    pub(crate) alignment: Alignment,
}

/// Render style set for full tabular data.
#[derive(Debug, Clone)]
pub(crate) struct RenderStyleSet {
    pub(crate) header: RenderStyle,
    pub(crate) data: RenderStyle,
    pub(crate) sum: RenderStyle,
}
