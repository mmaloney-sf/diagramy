// Color utilities for diagram rendering

/// RGB color representation
#[derive(Debug, Clone, Copy)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    /// Create a new RGB color
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        RgbColor { r, g, b }
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Parse from hex string
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');

        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(RgbColor { r, g, b })
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(RgbColor { r, g, b })
        } else {
            None
        }
    }

    /// Calculate relative luminance using sRGB formula
    /// https://www.w3.org/TR/WCAG20/#relativeluminancedef
    pub fn luminance(&self) -> f32 {
        let r_linear = (self.r as f32 / 255.0).powf(2.2);
        let g_linear = (self.g as f32 / 255.0).powf(2.2);
        let b_linear = (self.b as f32 / 255.0).powf(2.2);

        0.2126 * r_linear + 0.7152 * g_linear + 0.0722 * b_linear
    }
}

/// Calculate a contrasting RGB color for text that will be readable against the given background
/// Takes an RGB color and returns an RGB color that contrasts well with it
pub fn contrast(background: RgbColor) -> RgbColor {
    let luminance = background.luminance();

    // If background is light (high luminance), return dark text
    // If background is dark (low luminance), return light text
    if luminance > 0.5 {
        // Dark blue-gray for light backgrounds
        RgbColor::new(44, 62, 80)
    } else {
        // Soft white for dark backgrounds
        RgbColor::new(248, 249, 250)
    }
}

