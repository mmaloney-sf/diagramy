// Diagram representation with absolute coordinates

use svg::Document as SvgDocument;
use svg::node::element::{Rectangle, Text};
use crate::elaboration;

// Minimum font size as a fraction of the base font size
const MIN_FONTSIZE: f64 = 0.8;

/// A diagram containing positioned boxes with absolute coordinates
#[derive(Debug)]
pub struct Diagram {
    pub boxes: Vec<DiagramBox>,
    pub title: Option<String>,
    pub color: Option<String>,
}

/// A box in the diagram with absolute position and size
#[derive(Debug)]
pub struct DiagramBox {
    /// Absolute position (x, y) in the diagram coordinate space
    pub pos: (usize, usize),
    /// Absolute size (width, height) in the diagram coordinate space
    pub size: (usize, usize),
    pub title: Option<String>,
    pub color: Option<String>,
    /// Font scale factor based on width relative to parent
    pub font_scale: f64,
}

// Scale factor for border radius relative to font size
const BORDER_RADIUS_SCALE: f64 = 10.0;

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
    fn luminance(&self) -> f32 {
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

// Calculate a high-contrast text color based on background color (hex string version)
fn get_contrast_text_color(hex_color: &str) -> String {
    // Handle special case of transparent
    if hex_color == "transparent" {
        return "#2C3E50".to_string(); // Default to dark text
    }

    // Parse the hex color
    if let Some(bg_color) = RgbColor::from_hex(hex_color) {
        contrast(bg_color).to_hex()
    } else {
        // Fallback to dark text if parsing fails
        "#2C3E50".to_string()
    }
}


impl Diagram {
    /// Render the diagram to an SVG file
    ///
    /// # Arguments
    /// * `filename` - Path to the output SVG file
    /// * `width` - Width of the SVG canvas
    /// * `height` - Height of the SVG canvas
    /// * `font_size` - Font size for text rendering (default: 18)
    pub fn render_to_svg(&self, filename: &str, width: usize, height: usize, font_size: usize) -> Result<(), String> {
        // Create SVG document
        let mut svg_doc = SvgDocument::new()
            .set("width", width)
            .set("height", height)
            .set("viewBox", (0, 0, width, height));

        // Add background if diagram has a color
        if let Some(ref color) = self.color {
            let bg_color = crate::map_color(color)?;
            let background = Rectangle::new()
                .set("width", "100%")
                .set("height", "100%")
                .set("fill", bg_color);
            svg_doc = svg_doc.add(background);
        }

        // Calculate border radius based on font size
        let scale = font_size as f64 / 18.0;
        let border_radius = (BORDER_RADIUS_SCALE * scale) as usize;

        // First pass: Render all box rectangles
        for diagram_box in &self.boxes {
            svg_doc = render_box_rectangle(svg_doc, diagram_box, border_radius)?;
        }

        // Second pass: Render all box titles on top
        for diagram_box in &self.boxes {
            svg_doc = render_box_title(svg_doc, diagram_box, font_size)?;
        }

        // Render diagram title in upper left if present (on top of everything)
        if let Some(ref title) = self.title {
            let title_font_size = (font_size as f64 * 1.5) as usize;
            let padding = 10;

            let title_text = Text::new(title)
                .set("x", padding)
                .set("y", title_font_size + padding)
                .set("font-size", title_font_size)
                .set("font-family", "Arial, sans-serif")
                .set("font-weight", "bold")
                .set("fill", "#2C3E50");

            svg_doc = svg_doc.add(title_text);
        }

        // Save to file
        svg::save(filename, &svg_doc)
            .map_err(|e| format!("Failed to save SVG file: {}", e))?;

        Ok(())
    }
}

/// Render a box rectangle to the SVG document
fn render_box_rectangle(
    mut svg_doc: SvgDocument,
    diagram_box: &DiagramBox,
    border_radius: usize,
) -> Result<SvgDocument, String> {
    let (x, y) = diagram_box.pos;
    let (width, height) = diagram_box.size;

    // Determine fill color
    let fill_color = if let Some(ref color) = diagram_box.color {
        crate::map_color(color)?
    } else {
        "transparent"
    };

    // Create rounded rectangle
    let rect = Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", width)
        .set("height", height)
        .set("rx", border_radius)
        .set("ry", border_radius)
        .set("fill", fill_color)
        .set("stroke", "#333")
        .set("stroke-width", 2);

    svg_doc = svg_doc.add(rect);
    Ok(svg_doc)
}

/// Render a box title to the SVG document
fn render_box_title(
    mut svg_doc: SvgDocument,
    diagram_box: &DiagramBox,
    font_size: usize,
) -> Result<SvgDocument, String> {
    // Only render if title is present
    if let Some(ref title) = diagram_box.title {
        let (x, y) = diagram_box.pos;
        let (width, height) = diagram_box.size;

        // Determine fill color for contrast calculation
        let fill_color = if let Some(ref color) = diagram_box.color {
            crate::map_color(color)?
        } else {
            "transparent"
        };

        // Calculate text color based on background
        let text_color = get_contrast_text_color(fill_color);

        // Center the text in the box
        let text_x = x + width / 2;
        let text_y = y + height / 2;

        // Scale font size based on box width relative to parent
        let scaled_font_size = (font_size as f64 * diagram_box.font_scale) as usize;

        let text = Text::new(title)
            .set("x", text_x)
            .set("y", text_y)
            .set("text-anchor", "middle")
            .set("dominant-baseline", "middle")
            .set("font-size", scaled_font_size)
            .set("font-family", "Arial, sans-serif")
            .set("fill", text_color);

        svg_doc = svg_doc.add(text);
    }

    Ok(svg_doc)
}


/// Convert an elaboration::Diagram to a diagram::Diagram with absolute coordinates
pub fn from_elaboration(elab_diagram: &elaboration::ElaboratedDiagram) -> Diagram {
    let mut boxes = Vec::new();

    // Calculate absolute positions for all boxes
    // Start at position (0, 0) with the diagram's size
    let (width, height) = elab_diagram.size;

    // Process the top-level box
    flatten_boxes(
        &elab_diagram.top,
        0,
        0,
        width,
        height,
        width, // canvas width for logarithmic scaling
        &mut boxes,
    );

    Diagram {
        boxes,
        title: elab_diagram.title.clone(),
        color: Some(elab_diagram.color.clone()),
    }
}

/// Recursively flatten hierarchical boxes into absolute-positioned boxes
fn flatten_boxes(
    box_def: &elaboration::BoxDef,
    parent_x: usize,
    parent_y: usize,
    parent_width: usize,
    parent_height: usize,
    canvas_width: usize,
    output: &mut Vec<DiagramBox>,
) {
    let (grid_cols, grid_rows) = box_def.grid;

    // Calculate cell size based on parent dimensions and grid
    let cell_width = parent_width / grid_cols;
    let cell_height = parent_height / grid_rows;

    // Process all child boxes
    for child_box in &box_def.boxes {
        let (grid_x, grid_y) = child_box.pos;

        // Calculate absolute position
        let abs_x = parent_x + (grid_x * cell_width);
        let abs_y = parent_y + (grid_y * cell_height);

        // For now, assume each box takes one grid cell
        // TODO: Support boxes that span multiple cells
        let box_width = cell_width;
        let box_height = cell_height;

        // Add 5% margin on each side
        let margin_x = (box_width as f64 * 0.05) as usize;
        let margin_y = (box_height as f64 * 0.05) as usize;

        let final_x = abs_x + margin_x;
        let final_y = abs_y + margin_y;
        let final_width = box_width.saturating_sub(2 * margin_x);
        let final_height = box_height.saturating_sub(2 * margin_y);

        // Extract color from child box definition
        let color = child_box.def.color.clone();

        // Calculate font scale using logarithmic formula:
        // y = 1 - (1 - min_fontsize) * ln(2 - x) / log(2)
        // where x is the box width relative to canvas width
        let x = final_width as f64 / canvas_width as f64;
        // Clamp x to avoid ln of negative or zero values
        let x_clamped = x.min(1.999).max(0.001);
        let font_scale = 1.0 - (1.0 - MIN_FONTSIZE) * (2.0 - x_clamped).ln() / 2.0_f64.ln();

        // Add this box to output
        output.push(DiagramBox {
            pos: (final_x, final_y),
            size: (final_width, final_height),
            title: child_box.def.title.clone(),
            color,
            font_scale,
        });

        // Recursively process children of this box
        // Use the box with margins for child positioning
        flatten_boxes(
            &child_box.def,
            final_x,
            final_y,
            final_width,
            final_height,
            canvas_width,
            output,
        );
    }
}
