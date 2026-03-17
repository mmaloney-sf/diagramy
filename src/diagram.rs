// Diagram representation with absolute coordinates

use svg::Document as SvgDocument;
use svg::node::element::{Rectangle, Text};
use crate::elaboration;

// Minimum font size as a fraction of the base font size
const MIN_FONTSIZE: f64 = 0.7;

// Default base font size
const DEFAULT_FONT_SIZE: usize = 18;

// Margin around the top-level box (in pixels)
// Should be large enough to fit the title font (1.5x base font size) plus padding
const TOP_LEVEL_MARGIN: usize = (DEFAULT_FONT_SIZE as f64 * 1.5) as usize + 20;

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
    /// Absolute position in the diagram coordinate space
    pub pos: (usize, usize),
    /// Absolute size (width, height) in the diagram coordinate space
    pub size: (usize, usize),
    pub title: Option<String>,
    pub color: Option<String>,
    /// Font scale factor based on width relative to parent
    pub font_scale: f64,
    /// Whether this box has child boxes
    pub has_children: bool,
    /// Border style: solid, none, dotted, or dashed
    pub border_style: Option<String>,
}

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

        // First pass: Render all box rectangles
        for diagram_box in &self.boxes {
            svg_doc = render_box_rectangle(svg_doc, diagram_box)?;
        }

        // Second pass: Render all box titles on top
        for diagram_box in &self.boxes {
            svg_doc = render_box_title(svg_doc, diagram_box, font_size)?;
        }

        // Render diagram title centered at the top if present (on top of everything)
        if let Some(ref title) = self.title {
            let title_font_size = (font_size as f64 * 1.5) as usize;
            let padding = 10;

            // Split title by newlines and render each line centered
            let lines: Vec<&str> = title.split('\n').collect();
            let center_x = width / 2;
            for (i, line) in lines.iter().enumerate() {
                let line_y = title_font_size + padding + (i * title_font_size);
                let title_text = Text::new(*line)
                    .set("x", center_x)
                    .set("y", line_y)
                    .set("text-anchor", "middle")
                    .set("font-size", title_font_size)
                    .set("font-family", "Arial, sans-serif")
                    .set("font-weight", "bold")
                    .set("fill", "#2C3E50");

                svg_doc = svg_doc.add(title_text);
            }
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
) -> Result<SvgDocument, String> {
    let (x, y) = diagram_box.pos;
    let (width, height) = diagram_box.size;

    // Determine fill color
    let fill_color = if let Some(ref color) = diagram_box.color {
        crate::map_color(color)?
    } else {
        "transparent"
    };

    // Calculate stroke width proportional to box size
    // Use the smaller dimension to ensure consistent appearance
    let min_dimension = width.min(height) as f64;
    let stroke_width = (min_dimension / 100.0).max(0.5).min(4.0);

    // Calculate border radius proportional to box size (linear scaling)
    // Use the smaller dimension and scale it down
    let border_radius = (min_dimension / 20.0).max(2.0).min(15.0);

    // Determine border style (default is "solid")
    let border_style = diagram_box.border_style.as_deref().unwrap_or("solid");

    // Create rounded rectangle with appropriate border styling
    let mut rect = Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", width)
        .set("height", height)
        .set("rx", border_radius)
        .set("ry", border_radius)
        .set("fill", fill_color);

    // Apply border style
    match border_style {
        "none" => {
            // Transparent border (no stroke)
            rect = rect.set("stroke", "transparent").set("stroke-width", 0);
        }
        "dotted" => {
            // Dotted border
            rect = rect
                .set("stroke", "#333")
                .set("stroke-width", stroke_width)
                .set("stroke-dasharray", format!("{},{}", stroke_width * 2.0, stroke_width * 2.0));
        }
        "dashed" => {
            // Dashed border
            rect = rect
                .set("stroke", "#333")
                .set("stroke-width", stroke_width)
                .set("stroke-dasharray", format!("{},{}", stroke_width * 6.0, stroke_width * 3.0));
        }
        _ => {
            // Default: solid border
            rect = rect
                .set("stroke", "#333")
                .set("stroke-width", stroke_width);
        }
    }

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

        // Scale font size based on box width relative to parent
        let scaled_font_size = (font_size as f64 * diagram_box.font_scale) as usize;

        // Split title by newlines
        let lines: Vec<&str> = title.split('\n').collect();

        // Calculate padding based on box size (matches border radius calculation)
        let min_dimension = width.min(height) as f64;
        let padding = (min_dimension / 20.0).max(2.0).min(15.0) as usize;

        // Calculate available space for text
        let available_width = if diagram_box.has_children {
            // For boxes with children, text is left-aligned with padding on both sides
            width.saturating_sub(2 * padding)
        } else {
            // For boxes without children, text is centered but still needs padding
            width.saturating_sub(2 * padding)
        };
        let available_height = height.saturating_sub(2 * padding);

        // Estimate text dimensions and calculate scaling factor
        // Average character width is approximately 0.6 * font_size for Arial
        const CHAR_WIDTH_RATIO: f64 = 0.6;

        // Find the widest line
        let max_line_chars = lines.iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);

        // Calculate required width for the widest line
        let estimated_text_width = (max_line_chars as f64 * scaled_font_size as f64 * CHAR_WIDTH_RATIO) as usize;

        // Calculate required height for all lines
        let estimated_text_height = lines.len() * scaled_font_size;

        // Calculate scaling factors needed to fit within available space
        let width_scale = if estimated_text_width > available_width && estimated_text_width > 0 {
            available_width as f64 / estimated_text_width as f64
        } else {
            1.0
        };

        let height_scale = if estimated_text_height > available_height && estimated_text_height > 0 {
            available_height as f64 / estimated_text_height as f64
        } else {
            1.0
        };

        // Use the smaller of the two scaling factors to ensure text fits in both dimensions
        let final_scale = width_scale.min(height_scale);
        let final_font_size = (scaled_font_size as f64 * final_scale).max(1.0) as usize;

        // Position the text based on whether the box has children
        if diagram_box.has_children {
            // Box has children: position title in upper left
            let start_x = x + padding;
            let start_y = y + final_font_size + padding;

            // Render each line separately
            for (i, line) in lines.iter().enumerate() {
                let line_y = start_y + (i * final_font_size);
                let text = Text::new(*line)
                    .set("x", start_x)
                    .set("y", line_y)
                    .set("text-anchor", "start")
                    .set("dominant-baseline", "auto")
                    .set("font-size", final_font_size)
                    .set("font-family", "Arial, sans-serif")
                    .set("fill", text_color.clone());
                svg_doc = svg_doc.add(text);
            }
        } else {
            // Box has no children: center the text
            let center_x = x + width / 2;
            let center_y = y + height / 2;

            // Calculate total height of all lines
            let total_height = lines.len() * final_font_size;
            let start_y = center_y - (total_height / 2) + final_font_size;

            // Render each line centered
            for (i, line) in lines.iter().enumerate() {
                let line_y = start_y + (i * final_font_size);
                let text = Text::new(*line)
                    .set("x", center_x)
                    .set("y", line_y)
                    .set("text-anchor", "middle")
                    .set("dominant-baseline", "auto")
                    .set("font-size", final_font_size)
                    .set("font-family", "Arial, sans-serif")
                    .set("fill", text_color.clone());
                svg_doc = svg_doc.add(text);
            }
        }
    }

    Ok(svg_doc)
}


/// Convert an elaboration::Diagram to a diagram::Diagram with absolute coordinates
pub fn from_elaboration(elab_diagram: &elaboration::ElaboratedDiagram) -> Diagram {
    let mut boxes = Vec::new();

    // Calculate absolute positions for all boxes
    // Add margin around the top-level box
    let (canvas_width, canvas_height) = elab_diagram.size;
    let margin = TOP_LEVEL_MARGIN;

    // The top-level box starts at (margin, margin) and has reduced size
    let top_x = margin;
    let top_y = margin;
    let top_width = canvas_width.saturating_sub(2 * margin);
    let top_height = canvas_height.saturating_sub(2 * margin);

    // Process the top-level box
    flatten_boxes(
        &elab_diagram.top,
        top_x,
        top_y,
        top_width,
        top_height,
        canvas_width, // canvas width for logarithmic scaling
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
    // First, add the current box itself (if it has a title, color, or children)
    // Boxes with children should always be rendered to show their border
    if box_def.title.is_some() || box_def.color.is_some() || !box_def.boxes.is_empty() {
        // Linear scaling based on box width relative to canvas
        let width_ratio = parent_width as f64 / canvas_width as f64;
        let width_ratio_clamped = width_ratio.min(1.0).max(0.0);
        // Scale linearly from MIN_FONTSIZE to 1.0 based on width
        let font_scale = MIN_FONTSIZE + (1.0 - MIN_FONTSIZE) * width_ratio_clamped;

        output.push(DiagramBox {
            pos: (parent_x, parent_y),
            size: (parent_width, parent_height),
            title: box_def.title.clone(),
            color: box_def.color.clone(),
            font_scale,
            has_children: !box_def.boxes.is_empty(),
            border_style: box_def.border_style.clone(),
        });
    }

    // If this box has a title and children, add padding on all sides for the title
    let (padding_top, padding_left, padding_right, padding_bottom) = if box_def.title.is_some() && !box_def.boxes.is_empty() {
        // Calculate padding based on box size (matches border radius calculation)
        let min_dimension = parent_width.min(parent_height) as f64;
        let padding = (min_dimension / 20.0).max(2.0).min(15.0) as usize;

        // Apply margin scaling if specified
        let margin_scale = box_def.margin.unwrap_or(1.0);
        (
            (padding as f64 * margin_scale) as usize,
            (padding as f64 * margin_scale) as usize,
            (padding as f64 * margin_scale) as usize,
            (padding as f64 * margin_scale) as usize,
        )
    } else {
        (0, 0, 0, 0)
    };

    let (grid_rows, grid_cols) = box_def.grid;

    // Calculate cell size based on parent dimensions and grid
    // Subtract padding from available space
    let available_width = parent_width.saturating_sub(padding_left + padding_right);
    let available_height = parent_height.saturating_sub(padding_top + padding_bottom);
    let cell_width = available_width / grid_cols;
    let cell_height = available_height / grid_rows;

    // Process all child boxes
    for child_box in &box_def.boxes {
        let (grid_row, grid_col) = child_box.pos;
        let (span_height, span_width) = child_box.dim;

        // Calculate absolute position
        // Add padding to position to account for the title and side padding
        let abs_x = parent_x + padding_left + (grid_col * cell_width);
        let abs_y = parent_y + padding_top + (grid_row * cell_height);

        // Box spans multiple cells based on dim field
        let box_width = cell_width * span_width;
        let box_height = cell_height * span_height;

        // Use margin from box definition, defaulting to 0.1 (10%)
        // Margin is based on cell size (not box size) to ensure uniform margins regardless of dim
        let margin_factor = box_def.margin.unwrap_or(0.1);
        let margin_x = (cell_width as f64 * margin_factor) as usize;
        let margin_y = (cell_height as f64 * margin_factor) as usize;

        let final_x = abs_x + margin_x;
        let final_y = abs_y + margin_y;
        let final_width = box_width.saturating_sub(2 * margin_x);
        let final_height = box_height.saturating_sub(2 * margin_y);

        // Recursively process this box and its children
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
