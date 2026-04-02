// SVG rendering for diagrams

use svg::node::element::Group;
use svg::Document as SvgDocument;
use svg::node::element::{Rectangle, Text, Circle, Line, Marker, Polygon, Definitions};
use crate::diagram::{self, Diagram, DiagramBox, DiagramLabel};
use crate::elaboration::BoxKind;
// TODO: Re-enable when these types exist
// use crate::diagram::{DiagramArrow, DiagramElement, DiagramPort};

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
    dbg!(diagram);
    // Create SVG document
    let mut svg_doc = SvgDocument::new()
        .set("width", width)
        .set("height", height)
        .set("viewBox", (0, 0, width, height));

    // Add background if diagram has a color
    // TODO: Extract color from diagram.elements
    // if let Some(ref color) = diagram.color {
    //     let bg_color = crate::map_color(color)?;
    //     let background = Rectangle::new()
    //         .set("width", "100%")
    //         .set("height", "100%")
    //         .set("fill", bg_color);
    //     svg_doc = svg_doc.add(background);
    // }

    // Create the content group and apply vertical flip
    let content_group = create_content_group(diagram, width, height, font_size, debug)?.set("transform", "translate(0, 0)");

//    if debug {
//        content_group = content_group.set("transform", "translate(-100, -100)");
//    }

    svg_doc = svg_doc.add(content_group);


    Ok(svg_doc)
}

/// Create a group containing all diagram content with vertical flip applied
fn create_content_group(diagram: &Diagram, _width: usize, _height: usize, _font_size: usize, debug: bool) -> Result<svg::node::element::Group, String> {
    /*
    // Create a group that will contain all content
    let mut content_group = Group::new();

    for element in &diagram.elements {
        match element {
            crate::diagram::DiagramElement::Box(diagram_box) => {
                content_group = draw_box(content_group, &diagram_box, debug);
            },
            crate::diagram::DiagramElement::Port(diagram_port) => {
                content_group = draw_port(content_group, &diagram_port);
            },
            crate::diagram::DiagramElement::Arrow(_diagram_arrow) => {
                // Arrows are represented by their routed paths
                // The actual arrow element is just metadata
            },
            crate::diagram::DiagramElement::Path(points) => {
                content_group = draw_path(content_group, points);
            },
            crate::diagram::DiagramElement::Label(diagram_label) => {
                content_group = draw_label(content_group, &diagram_label, debug);
            }
            crate::diagram::DiagramElement::FillColor(_) => todo!(),
        }
    }

    Ok(content_group)
    */
    let mut content_group = Group::new();

    // Render the top box and its children recursively
    content_group = render_box_recursive(content_group, &diagram.top, debug)?;

    Ok(content_group)
}

/// Recursively render a box and its children
fn render_box_recursive(mut group: Group, diagram_box: &DiagramBox, debug: bool) -> Result<Group, String> {
    if diagram_box.boxdef.kind == BoxKind::Box {
        // Draw the box rectangle
        group = draw_box_rectangle(group, diagram_box, debug)?;
    }

    // Draw debug grid if enabled
    if debug {
        group = draw_debug_grid(group, diagram_box)?;
    }

    // Draw labels that were created in diagram.rs
    for label in &diagram_box.labels {
        group = draw_label(group, label, debug);
    }

    // Recursively render child boxes
    for child in &diagram_box.children {
        group = render_box_recursive(group, child, debug)?;
    }

    Ok(group)
}

fn draw_box_rectangle(mut group: Group, diagram_box: &DiagramBox, debug: bool) -> Result<Group, String> {
    let fill_color = if let Some(ref color) = diagram_box.boxdef.color {
        crate::map_color(color)?
    } else {
        "transparent"
    };
    let border_bounds = diagram_box.border_bounds();

//    let border_radius = box_rect.width() / 100.0;
    let rect = Rectangle::new()
        .set("x",      border_bounds.x())
        .set("y",      border_bounds.y())
        .set("width",  border_bounds.width())
        .set("height", border_bounds.height())
//        .set("rx", border_radius)
//        .set("ry", border_radius)
        .set("stroke", "blue")
        .set("stroke-width", 1.0)
        .set("fill", fill_color);

    group = group.add(rect);

    Ok(group)
}

fn draw_debug_grid(mut group: Group, diagram_box: &DiagramBox) -> Result<Group, String> {
    let grid_rect = diagram_box.grid_bounds();
    let (grid_rows, grid_cols) = diagram_box.boxdef.grid;
    let x = grid_rect.x();
    let y = grid_rect.y();
    let width = grid_rect.width();
    let height = grid_rect.height();

    let stroke_width = 0.70;

    // Draw red bounding rectangle around the grid
    let grid_bounds = Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", width)
        .set("height", height)
        .set("fill", "none")
        .set("stroke", "red")
        .set("stroke-width", stroke_width)
        .set("stroke-dasharray", "2,2");
    group = group.add(grid_bounds);

    // Draw vertical grid lines
    for col in 1..grid_cols {
        let x_pos = x + (col as f64 * width / grid_cols as f64);
        let line = Line::new()
            .set("x1", x_pos)
            .set("y1", y)
            .set("x2", x_pos)
            .set("y2", y + height)
            .set("stroke", "red")
            .set("stroke-width", stroke_width)
            .set("stroke-dasharray", "2,2");
        group = group.add(line);
    }

    // Draw horizontal grid lines
    for row in 1..grid_rows {
        let y_pos = y + (row as f64 * height / grid_rows as f64);
        let line = Line::new()
            .set("x1", x)
            .set("y1", y_pos)
            .set("x2", x + width)
            .set("y2", y_pos)
            .set("stroke", "red")
            .set("stroke-width", stroke_width)
            .set("stroke-dasharray", "2,2");
        group = group.add(line);
    }

    Ok(group)
}

fn draw_box(mut content: Group, diagram_box: &DiagramBox, debug: bool) -> Group {
    /*
    let fill_color = if let Some(ref color) = diagram_box.color {
        crate::map_color(color).unwrap()
    } else {
        "transparent"
    };

//    let slot_rect = Rectangle::new()
//        .set("x",      diagram_box.bounds().x())
//        .set("y",      diagram_box.bounds().y())
//        .set("width",  diagram_box.bounds().width())
//        .set("height", diagram_box.bounds().height())
//        .set("stroke", "blue")
//        .set("stroke-width", 1.0)
//        .set("fill", "#00ff0011");
//
//    content = content.add(slot_rect);

    let box_rect = diagram_box.border_bounds();

    let border_radius = diagram_box.margin / 3.0;
    let rect = Rectangle::new()
        .set("x",      box_rect.x())
        .set("y",      box_rect.y())
        .set("width",  box_rect.width())
        .set("height", box_rect.height())
        .set("rx", border_radius)
        .set("ry", border_radius)
        .set("stroke", "gray")
        .set("stroke-width", 1.0)
        .set("fill", fill_color);

//    match border_style {
//        "none" => {
//            // Transparent border (no stroke)
//            rect = rect
//                .set("stroke", "transparent")
//                .set("stroke-width", 0);
//        }
//        "dotted" => {
//            // Dotted border
//            rect = rect
//                .set("stroke", "#333")
//                .set("stroke-width", diagram_box.scaling())
//                .set("stroke-dasharray", "4,4");
//        }
//        "dashed" => {
//            // Dashed border
//            rect = rect
//                .set("stroke", "#333")
//                .set("stroke-width", diagram_box.scaling())
//                .set("stroke-dasharray", "12,6");
//        }
//        _ => {
//            // Default: solid border
//            rect = rect
//                .set("stroke", "#333")
//                .set("stroke-width", diagram_box.scaling());
//        }
//    }

    content = content.add(rect);

    if debug {
        let debug_grid_width = 2.0;
        // Draw grid overlay
        let grid_rect = diagram_box.grid_bounds();
        let (grid_rows, grid_cols) = diagram_box.grid;
        let x = grid_rect.x();
        let y = grid_rect.y();
        let width = grid_rect.width();
        let height = grid_rect.height();
        // Draw red bounding rectangle around the grid
        let grid_bounds = Rectangle::new()
            .set("x", x + 1.0)
            .set("y", y + 1.0)
            .set("width", width)
            .set("height", height)
            .set("fill", "none")
            .set("stroke", "red")
            .set("stroke-width", debug_grid_width);

        content = content.add(grid_bounds);

        // Draw vertical grid lines
        for col in 1..grid_cols {
            let x_pos = x + (col as f64 * width / grid_cols as f64);
            let line = svg::node::element::Line::new()
                .set("x1", x_pos + 1.0)
                .set("y1", y + 1.0)
                .set("x2", x_pos + 1.0)
                .set("y2", y + 1.0 + height)
                .set("stroke", "red")
                .set("stroke-width", debug_grid_width);
            content = content.add(line);
        }

        // Draw horizontal grid lines
        for row in 1..grid_rows {
            let y_pos = y + (row as f64 * height / grid_rows as f64);
            let line = svg::node::element::Line::new()
                .set("x1", x + 1.0)
                .set("y1", y_pos + 1.0)
                .set("x2", x + 1.0 + width)
                .set("y2", y_pos + 1.0)
                .set("stroke", "red")
                .set("stroke-width", debug_grid_width);
            content = content.add(line);
        }
    }

    content
*/
    todo!()
}

/// Calculate the bounding box for a multi-line text label at a given font size
///
/// # Arguments
/// * `text` - The text content (can contain newlines)
/// * `font_size` - The font size in pixels
///
fn draw_label(mut content: Group, diagram_label: &DiagramLabel, _debug: bool) -> Group {
    let box_rect = diagram_label.border_bounds();

    // Calculate font size to fill the available space
    let font_size = diagram::calculate_font_size_from_bounds(&diagram_label.text, box_rect);

    let lines: Vec<&str> = diagram_label.text.split('\n').collect();
    let center_x = box_rect.x() + box_rect.width() / 2.0;
    let center_y = box_rect.y() + box_rect.height() / 2.0;

    // Calculate total height of all lines
    let total_height = lines.len() as f64 * font_size;
    let start_y = center_y - (total_height / 2.0) + font_size;

    // Render each line centered
    for (i, line) in lines.iter().enumerate() {
        let line_y = start_y + (i as f64 * font_size);
        let text = Text::new(*line)
            .set("x", center_x)
            .set("y", line_y)
            .set("text-anchor", "middle")
            .set("dominant-baseline", "auto")
            .set("font-size", font_size)
            .set("font-family", "Arial, sans-serif")
            .set("fill", "black");
        content = content.add(text);
    }

    content
}

/*
fn draw_port(mut content: Group, diagram_port: &DiagramPort) -> Group {
    let (x, y) = diagram_port.pos;
    let radius = 5.0; // Port circle radius

    // Draw port as a circle
    let circle = svg::node::element::Circle::new()
        .set("cx", x)
        .set("cy", y)
        .set("r", radius)
        .set("fill", "red")
        .set("stroke", "darkred")
        .set("stroke-width", 1.5);

    content = content.add(circle);

    // Add label if present
    if let Some(ref label) = diagram_port.label {
        let label_offset = 8.0; // Distance from port center
        let text = Text::new(label.as_str())
            .set("x", x + label_offset)
            .set("y", y - label_offset)
            .set("font-size", 10)
            .set("font-family", "Arial, sans-serif")
            .set("fill", "black");
        content = content.add(text);
    }

    content
}

fn draw_path(mut content: Group, points: &[(f64, f64)]) -> Group {
    if points.is_empty() {
        return content;
    }

    // Build SVG path data
    let mut path_data = String::new();

    // Move to first point
    let (x0, y0) = points[0];
    path_data.push_str(&format!("M {},{}", x0, y0));

    // Line to subsequent points
    for &(x, y) in &points[1..] {
        path_data.push_str(&format!(" L {},{}", x, y));
    }

    // Create path element with arrow marker
    let path = svg::node::element::Path::new()
        .set("d", path_data)
        .set("stroke", "black")
        .set("stroke-width", 2)
        .set("fill", "none")
        .set("marker-end", "url(#arrowhead)");

    // Add arrowhead marker definition if not already added
    // (We'll add it once to the group)
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
                .set("fill", "black")
        );

    let defs = Definitions::new().add(marker);
    content = content.add(defs);
    content = content.add(path);

    content
}

    // First pass: Render all box rectangles
    // TODO: Extract boxes from diagram.elements
    // for diagram_box in &diagram.boxes {
    //     content_group = render_box_rectangle(content_group, diagram_box, debug)?;
    // }

//    // Second pass: Render all box titles on top
//    for diagram_box in &diagram.boxes {
//        content_group = render_box_title(content_group, diagram_box, font_size)?;
//    }
//
//    // Third pass: Render arrows (before ports so ports appear on top)
//    content_group = render_arrows(content_group, &diagram.arrows, &diagram.ports, &diagram.routed_paths)?;
//
//    // Fourth pass: Render ports as small circles
//    for port in &diagram.ports {
//        content_group = render_port(content_group, port)?;
//    }
//
//    // Render diagram title centered at the top if present (on top of everything)
//    if let Some(ref title) = diagram.title {
//        let title_font_size = (font_size as f64 * 1.5) as usize;
//        let padding = 10;
//
//        // Split title by newlines and render each line centered
//        let lines: Vec<&str> = title.split('\n').collect();
//        let center_x = width / 2;
//        for (i, line) in lines.iter().enumerate() {
//            let line_y = title_font_size + padding + (i * title_font_size);
//            let title_text = Text::new(*line)
//                .set("x", center_x)
//                .set("y", line_y)
//                .set("text-anchor", "middle")
//                .set("font-size", title_font_size)
//                .set("font-family", "Arial, sans-serif")
//                .set("font-weight", "bold")
//                .set("fill", "#2C3E50");
//
//            content_group = content_group.add(title_text);
//        }
//    }

    // Render debug grids on top of all other elements
//    for diagram_box in &diagram.boxes {
//        if diagram_box.debug {
//            content_group = render_debug_grid(content_group, diagram_box)?;
//        }
//    }

    // Add debug overlay if debug mode is enabled
    // TODO: Re-enable when diagram structure is restored
    // if debug {
    //     content_group = render_debug_overlay(content_group, diagram, width, height, font_size)?;
    // }

    // Apply vertical flip: scale(1, -1) translate(0, -height)
//    let transform = format!("scale(1, -1) translate(0, -{})", height);
//    content_group = content_group.set("transform", transform);
//
//    Ok(content_group)
//}

/// Render a box rectangle to a group
fn render_box_rectangle(
    mut group: svg::node::element::Group,
    diagram_box: &DiagramBox,
    _debug: bool,
) -> Result<svg::node::element::Group, String> {
    // Determine border style (default is "solid")
    // TODO: Re-enable when border_style field is available
    // let _border_style = diagram_box.border_style.as_deref().unwrap_or("solid");

    // Determine fill color
    let fill_color = if let Some(ref color) = diagram_box.color {
        crate::map_color(color)?
    } else {
        "transparent"
    };

//    let box_rect = diagram_box.bounds.scale_at_center(0.9);
    let box_rect = diagram_box.bounds;

    let border_radius = box_rect.width() / 100.0;
    let rect = Rectangle::new()
        .set("x",      box_rect.x())
        .set("y",      box_rect.y())
        .set("width",  box_rect.width())
        .set("height", box_rect.height())
        .set("rx", border_radius)
        .set("ry", border_radius)
        .set("stroke", "blue")
        .set("stroke-width", 1.0)
        .set("fill", fill_color);

//    let border_rect = diagram_box.border();
//
//    // Calculate border radius based on box dimensions
//    let min_dimension = border_rect.width().min(border_rect.height());
//    let border_radius = (min_dimension / 20.0).max(2.0).min(15.0);
//
//    let rect_border = Rectangle::new()
//        .set("x", border_rect.x())
//        .set("y", border_rect.y())
//        .set("width", border_rect.width())
//        .set("height", border_rect.height())
//        .set("rx", border_radius)
//        .set("ry", border_radius)
//        .set("fill", "blue")
//        .set("stroke", "blue")
//        .set("stroke-width", 1.0);
//
//    let grid_rect = diagram_box.grid();
//    let rect_grid = Rectangle::new()
//        .set("x", grid_rect.x())
//        .set("y", grid_rect.y())
//        .set("width", grid_rect.width())
//        .set("height", grid_rect.height())
//        .set("fill", "green")
//        .set("stroke", "green")
//        .set("stroke-width", diagram_box.scaling());

    // Apply border style
//    match border_style {
//        "none" => {
//            // Transparent border (no stroke)
//            rect = rect
//                .set("stroke", "transparent")
//                .set("stroke-width", 0);
//        }
//        "dotted" => {
//            // Dotted border
//            rect = rect
//                .set("stroke", "#333")
//                .set("stroke-width", diagram_box.scaling())
//                .set("stroke-dasharray", "4,4");
//        }
//        "dashed" => {
//            // Dashed border
//            rect = rect
//                .set("stroke", "#333")
//                .set("stroke-width", diagram_box.scaling())
//                .set("stroke-dasharray", "12,6");
//        }
//        _ => {
//            // Default: solid border
//            rect = rect
//                .set("stroke", "#333")
//                .set("stroke-width", diagram_box.scaling());
//        }
//    }

    group = group.add(rect);
//    group = group.add(rect_border);
//    group = group.add(rect_grid);

    // Draw red grid lines and bounding rectangle over the box
    // TODO: Re-enable when grid field is available on DiagramBox
    // let grid_rect = box_rect;//.scale_at_center(0.85);
    // let (grid_rows, grid_cols) = diagram_box.grid;
    // let x = grid_rect.x();
    // let y = grid_rect.y();
    // let width = grid_rect.width();
    // let height = grid_rect.height();
    //
    // // Draw red bounding rectangle around the grid
    // let grid_bounds = Rectangle::new()
    //     .set("x", x)
    //     .set("y", y)
    //     .set("width", width)
    //     .set("height", height)
    //     .set("fill", "none")
    //     .set("stroke", "red")
    //     .set("stroke-width", 2.0);
    // group = group.add(grid_bounds);
    //
    // // Draw vertical grid lines
    // for col in 1..grid_cols {
    //     let x_pos = x + (col as f64 * width / grid_cols as f64);
    //     let line = svg::node::element::Line::new()
    //         .set("x1", x_pos)
    //         .set("y1", y)
    //         .set("x2", x_pos)
    //         .set("y2", y + height)
    //         .set("stroke", "red")
    //         .set("stroke-width", 1.0);
    //     group = group.add(line);
    // }
    //
    // // Draw horizontal grid lines
    // for row in 1..grid_rows {
    //     let y_pos = y + (row as f64 * height / grid_rows as f64);
    //     let line = svg::node::element::Line::new()
    //         .set("x1", x)
    //         .set("y1", y_pos)
    //         .set("x2", x + width)
    //         .set("y2", y_pos)
    //         .set("stroke", "red")
    //         .set("stroke-width", 1.0);
    //     group = group.add(line);
    // }

    Ok(group)
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

/// Render a box title to a group
// TODO: Re-enable when DiagramBox fields are restored
#[allow(dead_code)]
fn render_box_title(
    group: svg::node::element::Group,
    _diagram_box: &DiagramBox,
    _font_size: usize,
) -> Result<svg::node::element::Group, String> {
    // Only render if title is present
    // if let Some(ref title) = diagram_box.title {
    //     let border = diagram_box.rect;
    //     let (x, y) = border.pos;
    //     let (width, height) = border.size;
    //
    //     // Determine fill color for contrast calculation
    //     let fill_color = if let Some(ref color) = diagram_box.color {
    //         crate::map_color(color)?
    //     } else {
    //         "transparent"
    //     };
    //
    //     // Calculate text color based on background
    //     let text_color = get_contrast_text_color(fill_color);
    //
    //     // Scale font size based on box width relative to parent
    //     let scaled_font_size = (font_size as f64 * diagram_box.font_scale) as usize;
    //
    //     // Split title by newlines
    //     let lines: Vec<&str> = title.split('\n').collect();
    //
    //     // Use the border dimensions directly - no manual padding calculation
    //     let available_width = width;
    //     let available_height = height;
    //
    //     // Estimate text dimensions and calculate scaling factor
    //     // Average character width is approximately 0.6 * font_size for Arial
    //     const CHAR_WIDTH_RATIO: f64 = 0.6;
    //
    //     // Find the widest line
    //     let max_line_chars = lines.iter()
    //         .map(|line| line.chars().count())
    //         .max()
    //         .unwrap_or(0);
    //
    //     // Calculate required width for the widest line
    //     let estimated_text_width = max_line_chars as f64 * scaled_font_size as f64 * CHAR_WIDTH_RATIO;
    //
    //     // Calculate required height for all lines
    //     let estimated_text_height = lines.len() as f64 * scaled_font_size as f64;
    //
    //     // Calculate scaling factors needed to fit within available space
    //     let width_scale = if estimated_text_width > available_width && estimated_text_width > 0.0 {
    //         available_width / estimated_text_width
    //     } else {
    //         1.0
    //     };
    //
    //     let height_scale = if estimated_text_height > available_height && estimated_text_height > 0.0 {
    //         available_height / estimated_text_height
    //     } else {
    //         1.0
    //     };
    //
    //     // Use the smaller of the two scaling factors to ensure text fits in both dimensions
    //     let final_scale = width_scale.min(height_scale);
    //     let final_font_size = (scaled_font_size as f64 * final_scale).max(1.0) as usize;
    //
    //     // Position the text based on whether the box has children
    //     if diagram_box.has_children {
    //         // Box has children: position title in upper left
    //         let start_x = x;
    //         let start_y = y + final_font_size as f64;
    //
    //         // Render each line separately
    //         for (i, line) in lines.iter().enumerate() {
    //             let line_y = start_y + (i as f64 * final_font_size as f64);
    //             let text = Text::new(*line)
    //                 .set("x", start_x)
    //                 .set("y", line_y)
    //                 .set("text-anchor", "start")
    //                 .set("dominant-baseline", "auto")
    //                 .set("font-size", final_font_size)
    //                 .set("font-family", "Arial, sans-serif")
    //                 .set("fill", text_color.clone());
    //             group = group.add(text);
    //         }
    //     } else {
    //         // Box has no children: center the text
    //         let center_x = x + width / 2.0;
    //         let center_y = y + height / 2.0;
    //
    //         // Calculate total height of all lines
    //         let total_height = lines.len() as f64 * final_font_size as f64;
    //         let start_y = center_y - (total_height / 2.0) + final_font_size as f64;
    //
    //         // Render each line centered
    //         for (i, line) in lines.iter().enumerate() {
    //             let line_y = start_y + (i as f64 * final_font_size as f64);
    //             let text = Text::new(*line)
    //                 .set("x", center_x)
    //                 .set("y", line_y)
    //                 .set("text-anchor", "middle")
    //                 .set("dominant-baseline", "auto")
    //                 .set("font-size", final_font_size)
    //                 .set("font-family", "Arial, sans-serif")
    //                 .set("fill", text_color.clone());
    //             group = group.add(text);
    //         }
    //     }
    // }

    Ok(group)
}

/// Render a port as a small circle with optional label to a group
#[allow(dead_code)]
fn render_port(mut group: svg::node::element::Group, port: &DiagramPort) -> Result<svg::node::element::Group, String> {
    let (x, y) = port.pos;
    let radius = 5;

    let circle = Circle::new()
        .set("cx", x)
        .set("cy", y)
        .set("r", radius)
        .set("fill", "#333")
        .set("stroke", "#333")
        .set("stroke-width", 2);

    group = group.add(circle);

    // Only render label if the port has a body with a label inside it
    // Don't render the port name
    if port.label.is_none() {
        return Ok(group);
    }

    let label_text = port.label.as_ref().unwrap();
    let font_size = 12;
    let offset = 10.0; // Distance from port center
    let char_width = font_size as f64 * 0.6; // Approximate character width

    // Get parent box boundaries
    // TODO: Re-enable when parent_rect is available
    // let box_x = port.parent_rect.x();
    // let box_y = port.parent_rect.y();
    // let box_right = port.parent_rect.right();
    // let box_bottom = port.parent_rect.bottom();
    let box_x = 0.0;
    let box_y = 0.0;
    let box_right = 100.0;
    let box_bottom = 100.0;

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
        group = group.add(text);
    }

    Ok(group)
}

/// Render arrows connecting ports using routed paths to a group
#[allow(dead_code)]
fn render_arrows(
    mut group: svg::node::element::Group,
    arrows: &[DiagramArrow],
    ports: &[DiagramPort],
    routed_paths: &[Vec<(f64, f64)>],
) -> Result<svg::node::element::Group, String> {
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
    group = group.add(defs);

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
            group = group.add(path_elem);
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

                group = group.add(line);
            }
        }
    }

    Ok(group)
}

/// Render debug grid overlay for a box to a group
#[allow(dead_code)]
fn render_debug_grid(
    parent_group: svg::node::element::Group,
    _diagram_box: &DiagramBox,
) -> Result<svg::node::element::Group, String> {
    // use svg::node::element::Group;
    //
    // let border = diagram_box.rect;
    // let (x, y) = border.pos;
    // let (width, height) = border.size;
    // let (grid_rows, grid_cols) = diagram_box.grid;
    //
    // // Create a group for the debug grid
    // let mut debug_group = Group::new()
    //     .set("class", "debug-grid");
    //
    // // Calculate cell size based on the box's grid property
    // let cell_width = width / grid_cols as f64;
    // let cell_height = height / grid_rows as f64;
    //
    // let grid_color = "#FF0000"; // Red color for grid lines
    // let grid_opacity = 0.3; // 30% opacity (70% transparent)
    // let debug_font_size = 10; // Small font size for debug numbers
    // let text_color = "#FF6666"; // Light red color for debug numbers
    //
    // // Draw vertical grid lines (grid_cols + 1 lines)
    // for i in 0..=grid_cols {
    //     let grid_x = x + (i as f64 * cell_width);
    //     let line = Line::new()
    //         .set("x1", grid_x)
    //         .set("y1", y)
    //         .set("x2", grid_x)
    //         .set("y2", y + height)
    //         .set("stroke", grid_color)
    //         .set("stroke-width", 1)
    //         .set("stroke-dasharray", "2,2")
    //         .set("opacity", grid_opacity);
    //     debug_group = debug_group.add(line);
    // }
    //
    // // Draw horizontal grid lines (grid_rows + 1 lines)
    // for i in 0..=grid_rows {
    //     let grid_y = y + (i as f64 * cell_height);
    //     let line = Line::new()
    //         .set("x1", x)
    //         .set("y1", grid_y)
    //         .set("x2", x + width)
    //         .set("y2", grid_y)
    //         .set("stroke", grid_color)
    //         .set("stroke-width", 1)
    //         .set("stroke-dasharray", "2,2")
    //         .set("opacity", grid_opacity);
    //     debug_group = debug_group.add(line);
    // }
    //
    // // Add column numbers above the box (1-indexed)
    // // Draw a single dark background rectangle for all column numbers (half as tall as cell height)
    // // Position it so the bottom is flush with the top of the box
    // // Extend it to the left by half the grid size to align with the row rectangle (creates a "corner" effect)
    // // Scale the height based on vertical scaling factor
    // let col_bg_height = (cell_height / 2.0) * diagram_box.vertical_scaling;
    // let col_bg_y = y - col_bg_height;
    // let col_bg_x = x - (cell_width / 2.0);
    // let col_bg_width = width + (cell_width / 2.0);
    // let col_bg_rect = Rectangle::new()
    //     .set("x", col_bg_x)
    //     .set("y", col_bg_y)
    //     .set("width", col_bg_width)
    //     .set("height", col_bg_height)
    //     .set("fill", "rgba(0, 0, 0, 1.0)");
    // debug_group = debug_group.add(col_bg_rect);
    // ... (rest of function body commented out)

    Ok(parent_group)
}

/// Render debug overlay with grid and labels to a group
#[allow(dead_code)]
fn render_debug_overlay(
    parent_group: svg::node::element::Group,
    _diagram: &Diagram,
    _width: usize,
    _height: usize,
    _font_size: usize,
) -> Result<svg::node::element::Group, String> {
    // use svg::node::element::Group;
    //
    // // Create a group for debug overlays
    // let mut debug_group = Group::new()
    //     .set("id", "debug-overlay");
    //
    // // Draw a grid overlay with 70% opacity
    // let grid_size = 50; // Grid cell size in pixels
    // let grid_color = "#FF0000"; // Red color for visibility
    // let grid_opacity = 0.3; // 30% opacity (70% transparent)
    // ... (rest of function body commented out)

    Ok(parent_group)
}

// COMMENTED OUT - requires Diagram fields that aren't available
/*
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
        let border = diagram_box.rect;
        let (x, y) = border.pos;
        let (box_width, box_height) = border.size;

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

    parent_group = parent_group.add(debug_group);
    Ok(parent_group)
}
*/
*/
