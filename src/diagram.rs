// Diagram representation with absolute coordinates

pub mod debug;

use svg::Document as SvgDocument;
use svg::node::element::{Rectangle, Text, Circle, Line, Marker, Polygon, Definitions};
use crate::{ast, elaboration};

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
    pub ports: Vec<DiagramPort>,
    pub arrows: Vec<DiagramArrow>,
    pub routed_paths: Vec<Vec<(f64, f64)>>, // Routed arrow paths in pixel coordinates
    pub title: Option<String>,
    pub color: Option<String>,
}

/// A port in the diagram with absolute position
#[derive(Debug)]
pub struct DiagramPort {
    pub name: String,
    pub pos: (f64, f64), // Absolute position
    pub label: Option<String>, // Optional label text
    pub parent_box: (f64, f64, f64, f64), // Parent box bounds (x, y, width, height)
}

/// An arrow in the diagram connecting two ports
#[derive(Debug)]
pub struct DiagramArrow {
    pub from: String,
    pub to: String,
}

/// A box in the diagram with absolute position and size
#[derive(Debug)]
pub struct DiagramBox {
    /// Absolute position in the diagram coordinate space
    pub pos: (f64, f64),
    /// Absolute size (width, height) in the diagram coordinate space
    pub size: (f64, f64),
    pub id: Option<String>,
    pub title: Option<String>,
    pub color: Option<String>,
    /// Font scale factor based on width relative to parent
    pub font_scale: f64,
    /// Whether this box has child boxes
    pub has_children: bool,
    /// Border style: solid, none, dotted, or dashed
    pub border_style: Option<String>,
    /// Horizontal scaling factor relative to top box (ratio: box width / top box width)
    pub horizontal_scaling: f64,
    /// Vertical scaling factor relative to top box (ratio: box height / top box height)
    pub vertical_scaling: f64,
    /// Whether to show debug grid overlay
    pub debug: bool,
    /// Grid dimensions (rows, cols) for debug overlay
    pub grid: (usize, usize),
    /// Name of the box definition (None for inline boxes)
    pub def_name: Option<String>,
    /// Line number where the box was defined
    pub line_number: Option<usize>,
}

impl DiagramBox {
    /// Returns the average scaling factor (average of horizontal and vertical scaling)
    pub fn scaling(&self) -> f64 {
        (self.horizontal_scaling + self.vertical_scaling) / 2.0
    }
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
    pub fn render_to_svg(&self, filename: &str, width: usize, height: usize, font_size: usize, debug: bool) -> Result<(), String> {
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

        // Third pass: Render arrows (before ports so ports appear on top)
        svg_doc = render_arrows(svg_doc, &self.arrows, &self.ports, &self.routed_paths)?;

        // Fourth pass: Render ports as small circles
        for port in &self.ports {
            svg_doc = render_port(svg_doc, port)?;
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

        // Render debug grids on top of all other elements
        for diagram_box in &self.boxes {
            if diagram_box.debug {
                svg_doc = render_debug_grid(svg_doc, diagram_box)?;
            }
        }

        // Add debug overlay if debug mode is enabled
        if debug {
            svg_doc = render_debug_overlay(svg_doc, self, width, height, font_size)?;
        }

        // Save to file
        svg::save(filename, &svg_doc)
            .map_err(|e| format!("Failed to save SVG file: {}", e))?;

        Ok(())
    }

    /// Render the diagram to an SVG string (for WebAssembly)
    ///
    /// # Arguments
    /// * `width` - Width of the SVG canvas
    /// * `height` - Height of the SVG canvas
    /// * `font_size` - Font size for text rendering (default: 18)
    /// * `debug` - Whether to include debug overlay
    #[cfg(all(target_arch = "wasm32", feature = "wasm-bindgen"))]
    pub fn render_to_svg_string(&self, width: usize, height: usize, font_size: usize, debug: bool) -> Result<String, String> {
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

        // Third pass: Render arrows (before ports so ports appear on top)
        svg_doc = render_arrows(svg_doc, &self.arrows, &self.ports, &self.routed_paths)?;

        // Fourth pass: Render ports as small circles
        for port in &self.ports {
            svg_doc = render_port(svg_doc, port)?;
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

        // Render debug grids on top of all other elements
        for diagram_box in &self.boxes {
            if diagram_box.debug {
                svg_doc = render_debug_grid(svg_doc, diagram_box)?;
            }
        }

        // Add debug overlay if debug mode is enabled
        if debug {
            svg_doc = render_debug_overlay(svg_doc, self, width, height, font_size)?;
        }

        // Convert to string
        Ok(svg_doc.to_string())
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

    // Determine border style (default is "solid")
    let border_style = diagram_box.border_style.as_deref().unwrap_or("solid");

    // Create rectangle with appropriate border styling
    let mut rect = Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", width)
        .set("height", height)
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
                .set("stroke-width", diagram_box.scaling())
                .set("stroke-dasharray", "4,4");
        }
        "dashed" => {
            // Dashed border
            rect = rect
                .set("stroke", "#333")
                .set("stroke-width", diagram_box.scaling())
                .set("stroke-dasharray", "12,6");
        }
        _ => {
            // Default: solid border
            rect = rect
                .set("stroke", "#333")
                .set("stroke-width", diagram_box.scaling());
        }
    }

    svg_doc = svg_doc.add(rect);

    Ok(svg_doc)
}

/// Render debug grid overlay for a box
fn render_debug_grid(
    mut svg_doc: SvgDocument,
    diagram_box: &DiagramBox,
) -> Result<SvgDocument, String> {
    use svg::node::element::Group;

    let (x, y) = diagram_box.pos;
    let (width, height) = diagram_box.size;
    let (grid_rows, grid_cols) = diagram_box.grid;

    // Create a group for the debug grid
    let mut debug_group = Group::new()
        .set("class", "debug-grid");

    // Calculate cell size based on the box's grid property
    let cell_width = width / grid_cols as f64;
    let cell_height = height / grid_rows as f64;

    let grid_color = "#FF0000"; // Red color for grid lines
    let grid_opacity = 0.3; // 30% opacity (70% transparent)
    let debug_font_size = 10; // Small font size for debug numbers
    let text_color = "#FF6666"; // Light red color for debug numbers

    // Draw vertical grid lines (grid_cols + 1 lines)
    for i in 0..=grid_cols {
        let grid_x = x + (i as f64 * cell_width);
        let line = svg::node::element::Line::new()
            .set("x1", grid_x)
            .set("y1", y)
            .set("x2", grid_x)
            .set("y2", y + height)
            .set("stroke", grid_color)
            .set("stroke-width", 1)
            .set("stroke-dasharray", "2,2")
            .set("opacity", grid_opacity);
        debug_group = debug_group.add(line);
    }

    // Draw horizontal grid lines (grid_rows + 1 lines)
    for i in 0..=grid_rows {
        let grid_y = y + (i as f64 * cell_height);
        let line = svg::node::element::Line::new()
            .set("x1", x)
            .set("y1", grid_y)
            .set("x2", x + width)
            .set("y2", grid_y)
            .set("stroke", grid_color)
            .set("stroke-width", 1)
            .set("stroke-dasharray", "2,2")
            .set("opacity", grid_opacity);
        debug_group = debug_group.add(line);
    }

    // Add column numbers above the box (1-indexed)
    // Draw a single dark background rectangle for all column numbers (half as tall as cell height)
    // Position it so the bottom is flush with the top of the box
    // Extend it to the left by half the grid size to align with the row rectangle (creates a "corner" effect)
    // Scale the height based on vertical scaling factor
    let col_bg_height = (cell_height / 2.0) * diagram_box.vertical_scaling;
    let col_bg_y = y - col_bg_height;
    let col_bg_x = x - (cell_width / 2.0);
    let col_bg_width = width + (cell_width / 2.0);
    let col_bg_rect = Rectangle::new()
        .set("x", col_bg_x)
        .set("y", col_bg_y)
        .set("width", col_bg_width)
        .set("height", col_bg_height)
        .set("fill", "rgba(0, 0, 0, 1.0)");
    debug_group = debug_group.add(col_bg_rect);

    for col in 1..=grid_cols {
        let col_x = x + ((col as f64 - 0.5) * cell_width);
        // Position numbers in the vertical center of the background rectangle
        let col_y = col_bg_y + col_bg_height / 2.0;

        let text = Text::new(col.to_string())
            .set("x", col_x)
            .set("y", col_y)
            .set("text-anchor", "middle")
            .set("dominant-baseline", "middle")
            .set("font-size", debug_font_size)
            .set("font-family", "Arial, sans-serif")
            .set("fill", text_color);
        debug_group = debug_group.add(text);
    }

    // Add row numbers to the left of the box (1-indexed)
    // Draw a single dark background rectangle for all row numbers (half as wide as cell width)
    // Scale the width based on horizontal scaling factor
    let row_bg_width = (cell_width / 2.0) * diagram_box.horizontal_scaling;
    let row_bg_x = x - row_bg_width;
    let row_bg_rect = Rectangle::new()
        .set("x", row_bg_x)
        .set("y", y)
        .set("width", row_bg_width)
        .set("height", height)
        .set("fill", "rgba(0, 0, 0, 1.0)");
    debug_group = debug_group.add(row_bg_rect);

    for row in 1..=grid_rows {
        // Center the row numbers horizontally in the background rectangle
        let row_x = row_bg_x + row_bg_width / 2.0;
        let row_y = y + ((row as f64 - 0.5) * cell_height);

        let text = Text::new(row.to_string())
            .set("x", row_x)
            .set("y", row_y)
            .set("text-anchor", "middle")
            .set("dominant-baseline", "middle")
            .set("font-size", debug_font_size)
            .set("font-family", "Arial, sans-serif")
            .set("fill", text_color);
        debug_group = debug_group.add(text);
    }

    // Add debug label at bottom right showing box def name and line number
    // Aligned with the grid
    if let Some(line_num) = diagram_box.line_number {
        let label_text = if let Some(ref def_name) = diagram_box.def_name {
            format!("{} line {}", def_name, line_num)
        } else {
            format!("line {}", line_num)
        };

        let label_font_size = 10.0 * diagram_box.vertical_scaling;

        // Calculate approximate text dimensions
        let char_width = label_font_size * 0.6;
        let text_width = label_text.len() as f64 * char_width;
        let text_height = label_font_size;

        // Position in lower right corner, aligned with grid
        // The rectangle should be aligned with the grid cell boundaries
        let bg_width = text_width + 4.0;
        let bg_height = text_height + 4.0;

        // Align with the bottom-right grid cell
        let label_x = x + width - bg_width;
        let label_y = y + height - bg_height;

        // Add background rectangle
        let bg_rect = Rectangle::new()
            .set("x", label_x)
            .set("y", label_y)
            .set("width", bg_width)
            .set("height", bg_height)
            .set("fill", "rgba(0, 0, 0, 1.0)");
        debug_group = debug_group.add(bg_rect);

        // Add text label (positioned inside the rectangle)
        let label = Text::new(label_text)
            .set("x", label_x + 2.0)
            .set("y", label_y + bg_height - 2.0)
            .set("text-anchor", "start")
            .set("dominant-baseline", "auto")
            .set("font-size", label_font_size)
            .set("font-family", "Arial, sans-serif")
            .set("fill", text_color);
        debug_group = debug_group.add(label);
    }

    svg_doc = svg_doc.add(debug_group);
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
        let min_dimension = width.min(height);
        let padding = (min_dimension / 20.0).max(2.0).min(15.0);

        // Calculate available space for text
        let available_width = if diagram_box.has_children {
            // For boxes with children, text is left-aligned with padding on both sides
            width - (2.0 * padding)
        } else {
            // For boxes without children, text is centered but still needs padding
            width - (2.0 * padding)
        };
        let available_height = height - (2.0 * padding);

        // Estimate text dimensions and calculate scaling factor
        // Average character width is approximately 0.6 * font_size for Arial
        const CHAR_WIDTH_RATIO: f64 = 0.6;

        // Find the widest line
        let max_line_chars = lines.iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);

        // Calculate required width for the widest line
        let estimated_text_width = max_line_chars as f64 * scaled_font_size as f64 * CHAR_WIDTH_RATIO;

        // Calculate required height for all lines
        let estimated_text_height = lines.len() as f64 * scaled_font_size as f64;

        // Calculate scaling factors needed to fit within available space
        let width_scale = if estimated_text_width > available_width && estimated_text_width > 0.0 {
            available_width / estimated_text_width
        } else {
            1.0
        };

        let height_scale = if estimated_text_height > available_height && estimated_text_height > 0.0 {
            available_height / estimated_text_height
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
            let start_y = y + final_font_size as f64 + padding;

            // Render each line separately
            for (i, line) in lines.iter().enumerate() {
                let line_y = start_y + (i as f64 * final_font_size as f64);
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
            let center_x = x + width / 2.0;
            let center_y = y + height / 2.0;

            // Calculate total height of all lines
            let total_height = lines.len() as f64 * final_font_size as f64;
            let start_y = center_y - (total_height / 2.0) + final_font_size as f64;

            // Render each line centered
            for (i, line) in lines.iter().enumerate() {
                let line_y = start_y + (i as f64 * final_font_size as f64);
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
    let mut ports = Vec::new();
    let mut arrows = Vec::new();

    // Calculate absolute positions for all boxes
    // Add margin around the top-level box
    let (canvas_width, canvas_height) = elab_diagram.size;
    let margin = TOP_LEVEL_MARGIN as f64;

    // The top-level box starts at (margin, margin) and has reduced size
    let top_x = margin;
    let top_y = margin;
    let top_width = (canvas_width as f64) - (2.0 * margin);
    let top_height = (canvas_height as f64) - (2.0 * margin);

    // Store top box dimensions for scaling calculations
    let top_box_width = top_width;
    let top_box_height = top_height;

    // Process the top-level box
    // Top box always uses center alignment
    flatten_boxes(
        &elab_diagram.top,
        None, // Top-level box has no ID
        top_x,
        top_y,
        top_width,
        top_height,
        canvas_width as f64, // canvas width for font scaling
        top_box_width,
        top_box_height,
        &ast::Alignment::Center,
        &mut boxes,
        &mut ports,
    );

    // Collect all arrows from the top-level box
    collect_arrows(&elab_diagram.top, &mut arrows);

    // Collect routed paths and convert to pixel coordinates
    let mut routed_paths = Vec::new();
    collect_routed_paths(
        &elab_diagram.top,
        top_x,
        top_y,
        top_width,
        top_height,
        &mut routed_paths,
    );

    Diagram {
        boxes,
        ports,
        arrows,
        routed_paths,
        title: elab_diagram.title.clone(),
        color: Some(elab_diagram.color.clone()),
    }
}

/// Recursively collect and convert routed paths from fractional to pixel coordinates
fn collect_routed_paths(
    box_def: &elaboration::BoxInst,
    parent_x: f64,
    parent_y: f64,
    parent_width: f64,
    parent_height: f64,
    output: &mut Vec<Vec<(f64, f64)>>,
) {
    let (grid_height, grid_width) = box_def.grid;

    // Calculate padding if this box has a title and children
    let (padding_top, padding_left, padding_right, padding_bottom) = if box_def.title.is_some() && !box_def.boxes.is_empty() {
        let min_dimension = parent_width.min(parent_height);
        let padding = (min_dimension / 20.0).max(2.0).min(15.0);
        let margin_scale = box_def.margin.unwrap_or(1.0);
        (
            padding * margin_scale,
            padding * margin_scale,
            padding * margin_scale,
            padding * margin_scale,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };

    // Calculate available space after padding
    let available_width = parent_width - (padding_left + padding_right);
    let available_height = parent_height - (padding_top + padding_bottom);

    // Convert routed paths from this box
    for path in &box_def.routed_arrow_paths {
        let pixel_path: Vec<(f64, f64)> = path.iter().map(|(row, col)| {
            // Scale fractional coordinates to pixel coordinates
            let frac_y = row / grid_height as f64;
            let frac_x = col / grid_width as f64;

            // Map to available space, accounting for padding
            let abs_x = parent_x + padding_left + (frac_x * available_width);
            let abs_y = parent_y + padding_top + (frac_y * available_height);

            (abs_x, abs_y)
        }).collect();
        output.push(pixel_path);
    }

    // Recursively collect from child boxes
    for child_box in &box_def.boxes {
        // Calculate child box's absolute position and size
        let (child_row, child_col) = child_box.pos;
        let (child_height, child_width) = child_box.dim;

        let cell_width = available_width / grid_width as f64;
        let cell_height = available_height / grid_height as f64;

        let abs_x = parent_x + padding_left + (child_col as f64 * cell_width);
        let abs_y = parent_y + padding_top + (child_row as f64 * cell_height);

        let box_width = cell_width * child_width as f64;
        let box_height = cell_height * child_height as f64;

        // Apply margin (same logic as flatten_boxes)
        let margin_factor = box_def.margin.unwrap_or(0.1);
        let margin_x = cell_width * margin_factor;
        let margin_y = cell_height * margin_factor;

        let child_x = abs_x + margin_x;
        let child_y = abs_y + margin_y;
        let child_pixel_width = box_width - (2.0 * margin_x);
        let child_pixel_height = box_height - (2.0 * margin_y);

        collect_routed_paths(
            &child_box.def,
            child_x,
            child_y,
            child_pixel_width,
            child_pixel_height,
            output,
        );
    }
}

/// Recursively flatten hierarchical boxes into absolute-positioned boxes
fn flatten_boxes(
    box_def: &elaboration::BoxInst,
    box_id: Option<&str>,
    parent_x: f64,
    parent_y: f64,
    parent_width: f64,
    parent_height: f64,
    canvas_width: f64,
    top_box_width: f64,
    top_box_height: f64,
    alignment: &ast::Alignment,
    output: &mut Vec<DiagramBox>,
    ports_output: &mut Vec<DiagramPort>,
) {
    // Calculate the natural aspect ratio of this box based on its grid
    let (grid_rows, grid_cols) = box_def.grid;
    let natural_aspect_ratio = grid_rows as f64 / grid_cols as f64;

    // Calculate what the natural width and height would be at the top box scale
    // We use the top box as a reference unit
    let natural_width_at_top_scale = top_box_width / grid_cols as f64 * grid_cols as f64;
    let natural_height_at_top_scale = natural_width_at_top_scale * natural_aspect_ratio;

    // Calculate uniform scaling factor to fit this box in the allocated space
    // Use the minimum scaling to preserve aspect ratio
    let horizontal_ratio = parent_width / natural_width_at_top_scale;
    let vertical_ratio = parent_height / natural_height_at_top_scale;
    let uniform_scaling = horizontal_ratio.min(vertical_ratio);

    // Calculate actual box size based on uniform scaling to preserve aspect ratio
    let actual_width = natural_width_at_top_scale * uniform_scaling;
    let actual_height = natural_height_at_top_scale * uniform_scaling;

    // Position the box within the allocated space based on alignment
    let (offset_x, offset_y) = match alignment {
        ast::Alignment::Top => ((parent_width - actual_width) / 2.0, 0.0),
        ast::Alignment::Right => (parent_width - actual_width, (parent_height - actual_height) / 2.0),
        ast::Alignment::Bottom => ((parent_width - actual_width) / 2.0, parent_height - actual_height),
        ast::Alignment::Left => (0.0, (parent_height - actual_height) / 2.0),
        ast::Alignment::Center => ((parent_width - actual_width) / 2.0, (parent_height - actual_height) / 2.0),
    };
    let actual_x = parent_x + offset_x;
    let actual_y = parent_y + offset_y;

    // For legacy purposes, set both horizontal and vertical scaling to the same value
    let horizontal_scaling = uniform_scaling;
    let vertical_scaling = uniform_scaling;

    // First, add the current box itself (if it has a title, color, children, ports, or arrows)
    // Boxes with children, ports, or arrows should always be rendered to show their border
    if box_def.title.is_some()
        || box_def.color.is_some()
        || !box_def.boxes.is_empty()
        || !box_def.ports.is_empty()
        || !box_def.arrows.is_empty() {
        // Linear scaling based on box width relative to canvas
        let width_ratio = parent_width / canvas_width;
        let width_ratio_clamped = width_ratio.min(1.0).max(0.0);
        // Scale linearly from MIN_FONTSIZE to 1.0 based on width
        let font_scale = MIN_FONTSIZE + (1.0 - MIN_FONTSIZE) * width_ratio_clamped;

        output.push(DiagramBox {
            pos: (actual_x, actual_y),
            size: (actual_width, actual_height),
            id: box_id.map(|s| s.to_string()),
            title: box_def.title.clone(),
            color: box_def.color.clone(),
            font_scale,
            has_children: !box_def.boxes.is_empty(),
            border_style: box_def.border_style.clone(),
            horizontal_scaling,
            vertical_scaling,
            debug: box_def.debug.unwrap_or(false),
            grid: box_def.grid,
            def_name: box_def.def_name.clone(),
            line_number: box_def.line_number,
        });
    }

    // Use actual box dimensions for child positioning
    let box_width_for_children = actual_width;
    let box_height_for_children = actual_height;
    let box_x_for_children = actual_x;
    let box_y_for_children = actual_y;

    // If this box has a title and children, add padding on all sides for the title
    let (padding_top, padding_left, padding_right, padding_bottom) = if box_def.title.is_some() && !box_def.boxes.is_empty() {
        // Calculate padding based on actual box size (matches border radius calculation)
        let min_dimension = box_width_for_children.min(box_height_for_children);
        let padding = (min_dimension / 20.0).max(2.0).min(15.0);

        // Apply margin scaling if specified
        let margin_scale = box_def.margin.unwrap_or(1.0);
        (
            padding * margin_scale,
            padding * margin_scale,
            padding * margin_scale,
            padding * margin_scale,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };

    // Calculate cell size based on actual box dimensions and grid
    // Subtract padding from available space
    let available_width = box_width_for_children - (padding_left + padding_right);
    let available_height = box_height_for_children - (padding_top + padding_bottom);
    let cell_width = available_width / grid_cols as f64;
    let cell_height = available_height / grid_rows as f64;

    // Process all child boxes
    for child_box in &box_def.boxes {
        let (grid_row, grid_col) = child_box.pos;
        let (span_height, span_width) = child_box.dim;

        // Calculate absolute position
        // Add padding to position to account for the title and side padding
        let abs_x = box_x_for_children + padding_left + (grid_col as f64 * cell_width);
        let abs_y = box_y_for_children + padding_top + (grid_row as f64 * cell_height);

        // Box spans multiple cells based on dim field
        let box_width = cell_width * span_width as f64;
        let box_height = cell_height * span_height as f64;

        // Use margin from box definition, defaulting to 0.1 (10%)
        // Margin is based on cell size (not box size) to ensure uniform margins regardless of dim
        let margin_factor = box_def.margin.unwrap_or(0.1);
        let margin_x = cell_width * margin_factor;
        let margin_y = cell_height * margin_factor;

        let final_x = abs_x + margin_x;
        let final_y = abs_y + margin_y;
        let final_width = box_width - (2.0 * margin_x);
        let final_height = box_height - (2.0 * margin_y);

        // Recursively process this box and its children
        // Use the box with margins for child positioning
        flatten_boxes(
            &child_box.def,
            child_box.id.as_deref(),
            final_x,
            final_y,
            final_width,
            final_height,
            canvas_width,
            top_box_width,
            top_box_height,
            &child_box.alignment,
            output,
            ports_output,
        );
    }

    // Process ports for this box
    for port in &box_def.ports {
        // Calculate absolute position based on fractional coordinates
        // Port coordinates are (row, col) where row is y and col is x
        // Fractional coordinates range from (0.0, 0.0) to (grid_height, grid_width)
        // Need to scale by grid dimensions to get fractional position in box
        let (grid_height, grid_width) = box_def.grid;

        // Scale coordinates: divide by grid dimensions to get 0.0-1.0 range
        let frac_y = port.coords.0 / grid_height as f64;
        let frac_x = port.coords.1 / grid_width as f64;

        // Map to actual box dimensions, accounting for padding
        // Ports should be positioned within the available space (after padding)
        let abs_x = parent_x + padding_left + (frac_x * available_width); // col is x
        let abs_y = parent_y + padding_top + (frac_y * available_height); // row is y

        ports_output.push(DiagramPort {
            name: port.name.clone(),
            pos: (abs_x, abs_y),
            label: port.label.clone(),
            parent_box: (parent_x, parent_y, parent_width, parent_height),
        });
    }
}

/// Recursively collect all arrows from a box and its children
fn collect_arrows(box_def: &elaboration::BoxInst, output: &mut Vec<DiagramArrow>) {
    // Add arrows from this box
    for arrow in &box_def.arrows {
        output.push(DiagramArrow {
            from: arrow.from.clone(),
            to: arrow.to.clone(),
        });
    }

    // Recursively collect from child boxes
    for child_box in &box_def.boxes {
        collect_arrows(&child_box.def, output);
    }
}

/// Render a port as a small circle with optional label
fn render_port(mut svg_doc: SvgDocument, port: &DiagramPort) -> Result<SvgDocument, String> {
    let (x, y) = port.pos;
    let radius = 5;

    let circle = Circle::new()
        .set("cx", x)
        .set("cy", y)
        .set("r", radius)
        .set("fill", "#333")
        .set("stroke", "#333")
        .set("stroke-width", 2);

    svg_doc = svg_doc.add(circle);

    // Only render label if the port has a body with a label inside it
    // Don't render the port name
    if port.label.is_none() {
        return Ok(svg_doc);
    }

    let label_text = port.label.as_ref().unwrap();
    let font_size = 12;
    let offset = 10.0; // Distance from port center
    let char_width = font_size as f64 * 0.6; // Approximate character width

    // Get parent box boundaries
    let (box_x, box_y, box_width, box_height) = port.parent_box;
    let box_right = box_x + box_width;
    let box_bottom = box_y + box_height;

    // Split label by newlines
    let lines: Vec<&str> = label_text.split('\n').collect();

    // Calculate label dimensions
    let max_line_width = lines.iter()
        .map(|line| line.len() as f64 * char_width)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);
    let label_height = lines.len() as f64 * font_size as f64;

    // Determine label position, ensuring it stays within parent box
    let mut label_x = x + offset;
    let mut label_y = y + 4.0; // Slightly below center for better alignment

    // Check if label would extend beyond right edge of box
    if label_x + max_line_width > box_right {
        // Try positioning to the left of the port instead
        label_x = x - offset - max_line_width;

        // If still outside, clamp to box boundary
        if label_x < box_x {
            label_x = box_x;
        }
    }

    // Check if label would extend beyond bottom edge of box
    if label_y + label_height > box_bottom {
        // Position above the port instead
        label_y = y - label_height + 4.0;

        // If still outside, clamp to box boundary
        if label_y < box_y {
            label_y = box_y + font_size as f64;
        }
    }

    // Ensure label doesn't extend beyond left edge
    if label_x < box_x {
        label_x = box_x;
    }

    // Ensure label doesn't extend beyond top edge
    if label_y - (font_size as f64) < box_y {
        label_y = box_y + (font_size as f64);
    }

    // Render each line
    for (i, line) in lines.iter().enumerate() {
        let line_y = label_y + (i as f64 * font_size as f64);
        let text = Text::new(*line)
            .set("x", label_x)
            .set("y", line_y)
            .set("font-size", font_size)
            .set("font-family", "Arial, sans-serif")
            .set("fill", "#333");
        svg_doc = svg_doc.add(text);
    }

    Ok(svg_doc)
}

/// Render arrows connecting ports using routed paths
fn render_arrows(
    mut svg_doc: SvgDocument,
    arrows: &[DiagramArrow],
    ports: &[DiagramPort],
    routed_paths: &[Vec<(f64, f64)>],
) -> Result<SvgDocument, String> {
    // Build a map of port names to positions
    let mut port_map = std::collections::HashMap::new();
    for port in ports {
        port_map.insert(port.name.clone(), port.pos);
    }

    // Add arrowhead marker definition
    let marker = Marker::new()
        .set("id", "arrowhead")
        .set("markerWidth", 10)
        .set("markerHeight", 10)
        .set("refX", 9)
        .set("refY", 3)
        .set("orient", "auto")
        .set("markerUnits", "strokeWidth")
        .add(
            Polygon::new()
                .set("points", "0,0 0,6 9,3")
                .set("fill", "#333")
        );

    let defs = Definitions::new().add(marker);
    svg_doc = svg_doc.add(defs);

    // Render each arrow using routed paths
    for (i, arrow) in arrows.iter().enumerate() {
        // Check if we have a routed path for this arrow
        if i < routed_paths.len() && !routed_paths[i].is_empty() {
            // Use routed path
            let path = &routed_paths[i];
            let mut path_data = String::new();

            for (j, &(x, y)) in path.iter().enumerate() {
                if j == 0 {
                    path_data.push_str(&format!("M {} {} ", x, y));
                } else {
                    path_data.push_str(&format!("L {} {} ", x, y));
                }
            }

            let path_elem = svg::node::element::Path::new()
                .set("d", path_data)
                .set("stroke", "#333")
                .set("stroke-width", 2)
                .set("fill", "none")
                .set("marker-end", "url(#arrowhead)");
            svg_doc = svg_doc.add(path_elem);
        } else {
            // Fallback to straight line if no routed path
            if let (Some(&from_pos), Some(&to_pos)) = (port_map.get(&arrow.from), port_map.get(&arrow.to)) {
                let (x1, y1) = from_pos;
                let (x2, y2) = to_pos;

                let line = Line::new()
                    .set("x1", x1)
                    .set("y1", y1)
                    .set("x2", x2)
                    .set("y2", y2)
                    .set("stroke", "#333")
                    .set("stroke-width", 2)
                    .set("marker-end", "url(#arrowhead)");

                svg_doc = svg_doc.add(line);
            }
        }
    }

    Ok(svg_doc)
}

/// Estimates the bounding box (width, height) of text at a given font size
///
/// # Arguments
/// * `text` - The text to measure (can contain newlines)
/// * `font_size` - The font size in pixels
///
/// # Returns
/// A tuple (width, height) representing the estimated bounding box in pixels
///
/// # Notes
/// - Uses a character width ratio of 0.6 for Arial font (approximation)
/// - Width is based on the widest line
/// - Height is number of lines × font_size
pub fn estimate_text_bbox(text: &str, font_size: usize) -> (usize, usize) {
    // Average character width is approximately 0.6 × font_size for Arial
    const CHAR_WIDTH_RATIO: f64 = 0.6;

    let lines: Vec<&str> = text.split('\n').collect();

    // Find the widest line
    let max_line_chars = lines.iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);

    // Calculate width based on widest line
    let width = (max_line_chars as f64 * font_size as f64 * CHAR_WIDTH_RATIO) as usize;

    // Calculate height based on number of lines
    let height = lines.len() * font_size;

    (width, height)
}

/// Render debug overlay with grid and labels
fn render_debug_overlay(
    mut svg_doc: SvgDocument,
    diagram: &Diagram,
    width: usize,
    height: usize,
    _font_size: usize,
) -> Result<SvgDocument, String> {
    use svg::node::element::Group;

    // Create a group for debug overlays
    let mut debug_group = Group::new()
        .set("id", "debug-overlay");

    // Draw a grid overlay with 70% opacity
    let grid_size = 50; // Grid cell size in pixels
    let grid_color = "#FF0000"; // Red color for visibility
    let grid_opacity = 0.3; // 30% opacity (70% transparent)

    // Draw vertical grid lines
    let mut x = 0;
    while x <= width {
        let line = svg::node::element::Line::new()
            .set("x1", x)
            .set("y1", 0)
            .set("x2", x)
            .set("y2", height)
            .set("stroke", grid_color)
            .set("stroke-width", 1)
            .set("opacity", grid_opacity);
        debug_group = debug_group.add(line);
        x += grid_size;
    }

    // Draw horizontal grid lines
    let mut y = 0;
    while y <= height {
        let line = svg::node::element::Line::new()
            .set("x1", 0)
            .set("y1", y)
            .set("x2", width)
            .set("y2", y)
            .set("stroke", grid_color)
            .set("stroke-width", 1)
            .set("opacity", grid_opacity);
        debug_group = debug_group.add(line);
        y += grid_size;
    }

    // Label each box with its index
    for (i, diagram_box) in diagram.boxes.iter().enumerate() {
        let (x, y) = diagram_box.pos;
        let (box_width, box_height) = diagram_box.size;

        // Position label at top-left corner of the box
        let label_x = x + 5.0;
        let label_y = y + 15.0;

        // Create label text with box number and name/title if available
        let label_text = if let Some(ref id) = diagram_box.id {
            format!("Box #{} ({})", i, id)
        } else if let Some(ref title) = diagram_box.title {
            // Use first line of title if it's multi-line
            let first_line = title.lines().next().unwrap_or("");
            if first_line.len() > 20 {
                format!("Box #{} ({}...)", i, &first_line[..20])
            } else {
                format!("Box #{} ({})", i, first_line)
            }
        } else {
            format!("Box #{}", i)
        };

        let label = Text::new(label_text)
            .set("x", label_x)
            .set("y", label_y)
            .set("font-size", 12)
            .set("font-family", "monospace")
            .set("font-weight", "bold")
            .set("fill", "#FF0000")
            .set("opacity", 1.0);

        debug_group = debug_group.add(label);

        // Also add a small rectangle outline to highlight the box
        let highlight = Rectangle::new()
            .set("x", x)
            .set("y", y)
            .set("width", box_width)
            .set("height", box_height)
            .set("fill", "none")
            .set("stroke", "#FF0000")
            .set("stroke-width", 2)
            .set("stroke-dasharray", "5,5")
            .set("opacity", 0.5);

        debug_group = debug_group.add(highlight);
    }

    // Label each port with its index and name
    for (i, port) in diagram.ports.iter().enumerate() {
        let (x, y) = port.pos;

        // Position label slightly offset from the port
        let label_x = x + 10.0;
        let label_y = y - 5.0;

        // Create label text with port number and name
        let label_text = format!("Port #{} ({})", i, port.name);

        let label = Text::new(label_text)
            .set("x", label_x)
            .set("y", label_y)
            .set("font-size", 10)
            .set("font-family", "monospace")
            .set("font-weight", "bold")
            .set("fill", "#0000FF")
            .set("opacity", 1.0);

        debug_group = debug_group.add(label);
    }

    svg_doc = svg_doc.add(debug_group);
    Ok(svg_doc)
}
