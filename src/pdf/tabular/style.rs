use super::super::types::{Color, IndirectFontRef};

/// All purpose alignment type.
#[derive(Debug, Clone, Copy)]
pub enum Alignment {
    Center,
    Left,
    Right,
}

/// Track render style.
#[derive(Debug, Clone)]
pub struct RenderStyle {
    pub font: IndirectFontRef,
    pub size: i32,
    pub foreground: Color,
    pub background: Color,
    pub alignment: Alignment,
}

/// Render style set for full tabular data.
#[derive(Debug, Clone)]
pub struct RenderStyleSet {
    pub header: RenderStyle,
    pub data: RenderStyle,
    pub sum: RenderStyle,
}
