use super::types::*;

pub struct Dimensions {
    pub height: Mm,
    pub width: Mm,
}

/// DIN A4 in mm dimensions.
pub const DIN_A4: Dimensions = Dimensions {
    height: Mm(210.),
    width: Mm(297.),
};

pub static TTF_REGULAR: &'static [u8] = include_bytes!("../../assets/Roboto-Regular.ttf");
pub static TTF_BOLD: &'static [u8] = include_bytes!("../../assets/Roboto-Bold.ttf");
