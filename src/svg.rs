// SVG rendering for diagrams

use svg::Document as SvgDocument;
use svg::node::element::{Rectangle, Text, Circle, Line, Marker, Polygon, Definitions};
use crate::diagram::{Diagram, DiagramBox, DiagramPort, DiagramArrow};

/// Render the diagram to an SVG file
///
/// # Arguments
/// * `diagram` - The diagram to render
/// * `filename` - Path to the output SVG file
/// * `width` - Width of the SVG canvas
/// * `height` - Height of the SVG canvas
/// * `font_size` - Font size for text rendering (default: 18)
/// * `debug` - Whether to include debug overlay
pub fn render_to_svg(diagram: &Diagram, filename: &str, width: usize, height: usize, font_size: usize, debug: bool) -> Result<(), String> {
    let svg_doc = create_svg_document(diagram, width, height, font_size, debug)?;

    // Save to file
    svg::save(filename, &svg_doc)
        .map_err(|e| format!("Failed to save SVG file: {}", e))?;

    Ok(())
}

/// Render the diagram to an SVG string (for WebAssembly)
///
/// # Arguments
/// * `diagram` - The diagram to render
/// * `width` - Width of the SVG canvas
/// * `height` - Height of the SVG canvas
/// * `font_size` - Font size for text rendering (default: 18)
/// * `debug` - Whether to include debug overlay
#[cfg(all(target_arch = "wasm32", feature = "wasm-bindgen"))]
pub fn render_to_svg_string(diagram: &Diagram, width: usize, height: usize, font_size: usize, debug: bool) -> Result<String, String> {
    let svg_doc = create_svg_document(diagram, width, height, font_size, debug)?;
    Ok(svg_doc.to_string())
}

/// Create an SVG document from a diagram
fn create_svg_document(diagram: &Diagram, width: usize, height: usize, font_size: usize, debug: bool) -> Result<SvgDocument, String> {
    // Create SVG document
    let mut svg_doc = SvgDocument::new()
        .set("width", width)
        .set("height", height)
        .set("viewBox", (0, 0, width, height));

    // Add background if diagram has a color
    if let Some(ref color) = diagram.color {
        let bg_color = crate::map_color(color)?;
        let background = Rectangle::new()
            .set("width", "100%")
            .set("height", "100%")
            .set("fill", bg_color);
        svg_doc = svg_doc.add(background);
    }

    // First pass: Render all box rectangles
    for diagram_box in &diagram.boxes {
        svg_doc = render_box_rectangle(svg_doc, diagram_box)?;
    }

    // Second pass: Render all box titles on top
    for diagram_box in &diagram.boxes {
        svg_doc = render_box_title(svg_doc, diagram_box, font_size)?;
    }

    // Third pass: Render arrows (before ports so ports appear on top)
    svg_doc = render_arrows(svg_doc, &diagram.arrows, &diagram.ports, &diagram.routed_paths)?;

    // Fourth pass: Render ports as small circles
    for port in &diagram.ports {
        svg_doc = render_port(svg_doc, port)?;
    }

    // Render diagram title centered at the top if present (on top of everything)
    if let Some(ref title) = diagram.title {
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
    for diagram_box in &diagram.boxes {
        if diagram_box.debug {
            svg_doc = render_debug_grid(svg_doc, diagram_box)?;
        }
    }

    // Add debug overlay if debug mode is enabled
    if debug {
        svg_doc = render_debug_overlay(svg_doc, diagram, width, height, font_size)?;
    }

    Ok(svg_doc)
}

/// Render a box rectangle to the SVG document
fn render_box_rectangle(
    mut svg_doc: SvgDocument,
    diagram_box: &DiagramBox,
) -> Result<SvgDocument, String> {
    let (x, y) = diagram_box.rect.pos;
    let (width, height) = diagram_box.rect.size;

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

/// Calculate a high-contrast text color based on background color (hex string version)
fn get_contrast_text_color(hex_color: &str) -> String {
    use crate::color::{RgbColor, contrast};

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

/// Render a box title to the SVG document
fn render_box_title(
    mut svg_doc: SvgDocument,
    diagram_box: &DiagramBox,
    font_size: usize,
) -> Result<SvgDocument, String> {
    // Only render if title is present
    if let Some(ref title) = diagram_box.title {
        let (x, y) = diagram_box.rect.pos;
        let (width, height) = diagram_box.rect.size;

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
    let box_x = port.parent_rect.x();
    let box_y = port.parent_rect.y();
    let box_right = port.parent_rect.right();
    let box_bottom = port.parent_rect.bottom();

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

/// Render debug grid overlay for a box
fn render_debug_grid(
    mut svg_doc: SvgDocument,
    diagram_box: &DiagramBox,
) -> Result<SvgDocument, String> {
    use svg::node::element::Group;

    let (x, y) = diagram_box.rect.pos;
    let (width, height) = diagram_box.rect.size;
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
        let line = Line::new()
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
        let line = Line::new()
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
        let line = Line::new()
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
        let line = Line::new()
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
        let (x, y) = diagram_box.rect.pos;
        let (box_width, box_height) = diagram_box.rect.size;

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
