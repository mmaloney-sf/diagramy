// Diagram representation with absolute coordinates

use svg::Document as SvgDocument;
use svg::node::element::{Rectangle, Text, Circle, Line, Marker, Polygon, Definitions};
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
    pub ports: Vec<DiagramPort>,
    pub arrows: Vec<DiagramArrow>,
    pub routed_paths: Vec<Vec<(usize, usize)>>, // Routed arrow paths in pixel coordinates
    pub title: Option<String>,
    pub color: Option<String>,
}

/// A port in the diagram with absolute position
#[derive(Debug)]
pub struct DiagramPort {
    pub name: String,
    pub pos: (usize, usize), // Absolute position
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
    /// Horizontal scaling factor relative to top box (ratio: box width / top box width)
    pub horizontal_scaling: f64,
    /// Vertical scaling factor relative to top box (ratio: box height / top box height)
    pub vertical_scaling: f64,
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
    let mut ports = Vec::new();
    let mut arrows = Vec::new();

    // Calculate absolute positions for all boxes
    // Add margin around the top-level box
    let (canvas_width, canvas_height) = elab_diagram.size;
    let margin = TOP_LEVEL_MARGIN;

    // The top-level box starts at (margin, margin) and has reduced size
    let top_x = margin;
    let top_y = margin;
    let top_width = canvas_width.saturating_sub(2 * margin);
    let top_height = canvas_height.saturating_sub(2 * margin);

    // Store top box dimensions for scaling calculations
    let top_box_width = top_width as f64;
    let top_box_height = top_height as f64;

    // Process the top-level box
    flatten_boxes(
        &elab_diagram.top,
        top_x,
        top_y,
        top_width,
        top_height,
        canvas_width, // canvas width for font scaling
        top_box_width,
        top_box_height,
        &mut boxes,
        &mut ports,
    );

    // Collect all arrows from the top-level box
    collect_arrows(&elab_diagram.top, &mut arrows);

    // Collect routed paths and convert to pixel coordinates
    let routed_paths = convert_routed_paths(
        &elab_diagram.top,
        top_x,
        top_y,
        top_width,
        top_height,
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

/// Convert routed paths from fractional to pixel coordinates
fn convert_routed_paths(
    box_def: &elaboration::BoxDef,
    parent_x: usize,
    parent_y: usize,
    parent_width: usize,
    parent_height: usize,
) -> Vec<Vec<(usize, usize)>> {
    let (grid_height, grid_width) = box_def.grid;

    box_def.routed_arrow_paths.iter().map(|path| {
        path.iter().map(|(row, col)| {
            // Scale fractional coordinates to pixel coordinates
            let frac_y = row / grid_height as f64;
            let frac_x = col / grid_width as f64;

            let abs_x = parent_x + (frac_x * parent_width as f64) as usize;
            let abs_y = parent_y + (frac_y * parent_height as f64) as usize;

            (abs_x, abs_y)
        }).collect()
    }).collect()
}

/// Recursively flatten hierarchical boxes into absolute-positioned boxes
fn flatten_boxes(
    box_def: &elaboration::BoxDef,
    parent_x: usize,
    parent_y: usize,
    parent_width: usize,
    parent_height: usize,
    canvas_width: usize,
    top_box_width: f64,
    top_box_height: f64,
    output: &mut Vec<DiagramBox>,
    ports_output: &mut Vec<DiagramPort>,
) {
    // First, add the current box itself (if it has a title, color, or children)
    // Boxes with children should always be rendered to show their border
    if box_def.title.is_some() || box_def.color.is_some() || !box_def.boxes.is_empty() {
        // Linear scaling based on box width relative to canvas
        let width_ratio = parent_width as f64 / canvas_width as f64;
        let width_ratio_clamped = width_ratio.min(1.0).max(0.0);
        // Scale linearly from MIN_FONTSIZE to 1.0 based on width
        let font_scale = MIN_FONTSIZE + (1.0 - MIN_FONTSIZE) * width_ratio_clamped;

        // Calculate scaling factors relative to top box
        let horizontal_scaling = parent_width as f64 / top_box_width;
        let vertical_scaling = parent_height as f64 / top_box_height;

        output.push(DiagramBox {
            pos: (parent_x, parent_y),
            size: (parent_width, parent_height),
            title: box_def.title.clone(),
            color: box_def.color.clone(),
            font_scale,
            has_children: !box_def.boxes.is_empty(),
            border_style: box_def.border_style.clone(),
            horizontal_scaling,
            vertical_scaling,
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
            top_box_width,
            top_box_height,
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

        // Map to actual box dimensions
        let abs_x = parent_x + (frac_x * parent_width as f64) as usize; // col is x
        let abs_y = parent_y + (frac_y * parent_height as f64) as usize; // row is y

        ports_output.push(DiagramPort {
            name: port.name.clone(),
            pos: (abs_x, abs_y),
        });
    }
}

/// Recursively collect all arrows from a box and its children
fn collect_arrows(box_def: &elaboration::BoxDef, output: &mut Vec<DiagramArrow>) {
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

/// Render a port as a small circle
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
    Ok(svg_doc)
}

/// Render arrows connecting ports using routed paths
fn render_arrows(
    mut svg_doc: SvgDocument,
    arrows: &[DiagramArrow],
    ports: &[DiagramPort],
    routed_paths: &[Vec<(usize, usize)>],
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
