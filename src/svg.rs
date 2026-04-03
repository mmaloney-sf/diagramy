// SVG rendering for diagrams

use svg::node::element::Group;
use svg::Document as SvgDocument;
use svg::node::element::{Rectangle, Text, Circle, Line, Marker, Polygon, Definitions};
use crate::diagram::{self, Diagram, DiagramBox, DiagramLabel, DiagramPort};
use crate::elaboration::BoxKind;
// TODO: Re-enable when these types exist
// use crate::diagram::{DiagramArrow, DiagramElement};

pub fn render_to_svg(diagram: &Diagram, filename: &str, width: usize, height: usize, font_size: usize, debug: bool) -> Result<(), String> {
    let svg_doc = create_svg_document(diagram, width, height, font_size, debug)?;

    // Save to file
    svg::save(filename, &svg_doc)
        .map_err(|e| format!("Failed to save SVG file: {}", e))?;

    Ok(())
}

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
fn create_content_group(
    diagram: &Diagram,
    _width: usize,
    _height: usize,
     _font_size: usize,
    debug: bool,
) -> Result<svg::node::element::Group, String> {
    let mut content_group = Group::new();
    content_group = render_box(content_group, &diagram.top, debug)?;
    Ok(content_group)
}

fn draw_port(mut content: Group, diagram_port: &DiagramPort) -> Group {
    let (inner_x, inner_y) = diagram_port.pos_inner;
    let radius = 1.5; // Port circle radius

    if let Some((x, y)) = diagram_port.pos_outer {
        let circle = svg::node::element::Circle::new()
            .set("cx", x)
            .set("cy", y)
            .set("r", radius)
            .set("fill", "red")
            .set("stroke", "darkred")
            .set("stroke-width", 1.5);

        content = content.add(circle);
    }

    let inner_circle = svg::node::element::Circle::new()
        .set("cx", inner_x)
        .set("cy", inner_y)
        .set("r", radius)
        .set("fill", "red")
        .set("stroke", "darkred")
        .set("stroke-width", 1.5);

    content = content.add(inner_circle);

    // Add label if present
    if let Some(ref label) = diagram_port.label {
        let label_offset = 8.0; // Distance from port center
        let text = Text::new(label.as_str())
            .set("x", inner_x + label_offset)
            .set("y", inner_y - label_offset)
            .set("font-size", 10)
            .set("font-family", "Arial, sans-serif")
            .set("fill", "black");
        content = content.add(text);
    }

    content
}

/// Recursively render a box and its children
fn render_box(mut group: Group, diagram_box: &DiagramBox, debug: bool) -> Result<Group, String> {
    if diagram_box.boxdef.kind == BoxKind::Box {
        // Draw the box rectangle
        group = draw_box_rectangle(group, diagram_box, debug)?;
    }

    // Draw debug grid if enabled
    if debug || diagram_box.boxdef.debug.unwrap_or(false) {
        group = draw_debug_grid(group, diagram_box)?;
    }

    // Draw labels that were created in diagram.rs
    for label in &diagram_box.labels {
        group = draw_label(group, label, debug);
    }

    // Draw ports that were created in diagram.rs
    for port in &diagram_box.ports {
        group = draw_port(group, port);
    }

    // Recursively render child boxes
    for child in &diagram_box.children {
        group = render_box(group, child, debug)?;
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
    let stroke_width = 0.5;

//    let border_radius = box_rect.width() / 100.0;
    let rect = Rectangle::new()
        .set("x",      border_bounds.x())
        .set("y",      border_bounds.y())
        .set("width",  border_bounds.width())
        .set("height", border_bounds.height())
//        .set("rx", border_radius)
//        .set("ry", border_radius)
        .set("stroke", "gray")
        .set("stroke-width", stroke_width)
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
    let stroke_dasharray = "1,3";

    // Draw red bounding rectangle around the grid
    let grid_bounds = Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", width)
        .set("height", height)
        .set("fill", "none")
        .set("stroke", "red")
        .set("stroke-width", stroke_width)
        .set("stroke-dasharray", stroke_dasharray);
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
            .set("stroke-dasharray", stroke_dasharray);
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
            .set("stroke-dasharray", stroke_dasharray);
        group = group.add(line);
    }

    Ok(group)
}

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
