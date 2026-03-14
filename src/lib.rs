// Include the generated parser module
#[macro_use] extern crate lalrpop_util;

use svg::Document as SvgDocument;
use svg::node::element::{Text, Rectangle, Circle, Line, Path};
use svg::node::element::path::Data;
use std::collections::HashMap;

pub mod ast;
use ast::{Document, Box, Property, LayoutProperty, Port, PortProperty, Arrow};

lalrpop_mod!(pub grammar); // synthesized by LALRPOP

// Build a layout map from the layout section
// Positions in the layout are relative to parent, so we need to convert them to absolute
fn build_layout_map(doc: &Document) -> HashMap<String, (i32, i32, i32, i32)> {
    // First, build a map of relative positions from the layout section
    let mut relative_layout_map = HashMap::new();

    for item in &doc.layout.items {
        let mut pos = (0, 0);
        let mut size = (100, 50); // default size

        for prop in &item.properties {
            match prop {
                LayoutProperty::Pos(x, y) => pos = (*x, *y),
                LayoutProperty::Size(w, h) => size = (*w, *h),
                LayoutProperty::Interp(_) => {}, // Handled separately for ports
            }
        }

        // Store as (x, y, width, height) - these are relative positions
        relative_layout_map.insert(item.name.clone(), (pos.0, pos.1, size.0, size.1));
    }

    // Now convert relative positions to absolute by walking the diagram tree
    let mut absolute_layout_map = HashMap::new();
    for box_item in &doc.diagram.boxes {
        convert_to_absolute_positions(box_item, &relative_layout_map, &mut absolute_layout_map, 0, 0);
    }

    absolute_layout_map
}

// Recursively convert relative positions to absolute positions
fn convert_to_absolute_positions(
    box_item: &Box,
    relative_map: &HashMap<String, (i32, i32, i32, i32)>,
    absolute_map: &mut HashMap<String, (i32, i32, i32, i32)>,
    parent_x: i32,
    parent_y: i32,
) {
    if let Some(ref id) = box_item.id {
        if let Some(&(rel_x, rel_y, width, height)) = relative_map.get(id) {
            // Convert relative position to absolute
            let abs_x = parent_x + rel_x;
            let abs_y = parent_y + rel_y;

            // Store absolute position
            absolute_map.insert(id.clone(), (abs_x, abs_y, width, height));

            // Process children with this box's absolute position as their parent
            for child in &box_item.children {
                convert_to_absolute_positions(child, relative_map, absolute_map, abs_x, abs_y);
            }
        }
    }
}

// Map .dia color names to SVG hex color codes
// Colors chosen to match reference.png - muted, professional palette
pub fn map_color(color_name: &str) -> &str {
    match color_name {
        "red" => "#D98880",        // Soft coral red
        "blue" => "#85C1E2",       // Soft sky blue
        "green" => "#82E0AA",      // Soft mint green
        "yellow" => "#F9E79F",     // Soft pale yellow
        "orange" => "#F5B041",     // Soft orange
        "purple" => "#BB8FCE",     // Soft lavender purple
        "pink" => "#F5B7B1",       // Soft pastel pink
        "cyan" => "#7FB3D5",       // Soft cyan blue
        "magenta" => "#D7BDE2",    // Soft magenta/lilac
        "lime" => "#ABEBC6",       // Soft lime green
        "teal" => "#76D7C4",       // Soft teal
        "indigo" => "#A9CCE3",     // Soft indigo blue
        "brown" => "#C39BD3",      // Soft mauve
        "gray" => "#D5DBDB",       // Soft light gray
        "grey" => "#D5DBDB",       // Soft light gray
        "black" => "#566573",      // Soft dark gray
        "white" => "#F8F9F9",      // Soft white
        "navy" => "#5D6D7E",       // Soft navy gray
        "maroon" => "#C39BD3",     // Soft purple
        "olive" => "#A9DFBF",      // Soft olive green
        _ => "#D5DBDB",            // Default to soft gray for unknown colors
    }
}

// Calculate a high-contrast text color based on background color
// Uses relative luminance to determine if background is light or dark
// Returns eye-friendly colors instead of pure black/white
fn get_contrast_text_color(hex_color: &str) -> &str {
    // Parse hex color (supports both #RGB and #RRGGBB formats)
    let hex = hex_color.trim_start_matches('#');

    let (r, g, b) = if hex.len() == 6 {
        // Full hex format: #RRGGBB
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
        (r, g, b)
    } else if hex.len() == 3 {
        // Short hex format: #RGB
        let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(8) * 17;
        let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(8) * 17;
        let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(8) * 17;
        (r, g, b)
    } else {
        // Invalid format, default to medium gray
        (128, 128, 128)
    };

    // Calculate relative luminance using sRGB formula
    // https://www.w3.org/TR/WCAG20/#relativeluminancedef
    let r_linear = (r as f32 / 255.0).powf(2.2);
    let g_linear = (g as f32 / 255.0).powf(2.2);
    let b_linear = (b as f32 / 255.0).powf(2.2);

    let luminance = 0.2126 * r_linear + 0.7152 * g_linear + 0.0722 * b_linear;

    // Use eye-friendly colors instead of pure black/white
    // Dark backgrounds get a soft white, light backgrounds get a dark gray
    if luminance > 0.5 {
        "#2C3E50" // Dark blue-gray for light backgrounds
    } else {
        "#F8F9FA" // Soft white for dark backgrounds
    }
}

// Validate the diagram layout
fn validate_layout(doc: &Document, layout_map: &HashMap<String, (i32, i32, i32, i32)>) -> Result<(), String> {
    // Get canvas size
    let (canvas_width, canvas_height) = doc.layout.canvas_size.unwrap_or((800, 600));

    // Validate each top-level box and its children
    for box_item in &doc.diagram.boxes {
        validate_box(box_item, layout_map, None)?;
    }

    // Check for overlaps between sibling boxes at each level
    check_sibling_overlaps(&doc.diagram.boxes, layout_map, None)?;

    // Check that all boxes are within canvas bounds
    check_canvas_bounds(layout_map, canvas_width, canvas_height)?;

    Ok(())
}

// Validate a single box and its children
fn validate_box(
    box_item: &Box,
    layout_map: &HashMap<String, (i32, i32, i32, i32)>,
    parent_bounds: Option<(i32, i32, i32, i32)>, // (x, y, width, height)
) -> Result<(), String> {
    if let Some(ref id) = box_item.id {
        if let Some(&(x, y, width, height)) = layout_map.get(id) {
            // Check if this box is completely contained within its parent
            if let Some((px, py, pw, ph)) = parent_bounds {
                if x < px || y < py || x + width > px + pw || y + height > py + ph {
                    return Err(format!(
                        "Box '{}' at ({}, {}) with size ({}, {}) is not completely contained within its parent at ({}, {}) with size ({}, {})",
                        id, x, y, width, height, px, py, pw, ph
                    ));
                }
            }

            // Recursively validate children
            for child in &box_item.children {
                validate_box(child, layout_map, Some((x, y, width, height)))?;
            }

            // Check for overlaps between children
            check_sibling_overlaps(&box_item.children, layout_map, Some((x, y, width, height)))?;
        }
    }

    Ok(())
}

// Check if two boxes overlap
fn boxes_overlap(
    box1: (i32, i32, i32, i32), // (x1, y1, w1, h1)
    box2: (i32, i32, i32, i32), // (x2, y2, w2, h2)
) -> bool {
    let (x1, y1, w1, h1) = box1;
    let (x2, y2, w2, h2) = box2;

    // Two rectangles overlap if they overlap in both x and y dimensions
    let x_overlap = x1 < x2 + w2 && x1 + w1 > x2;
    let y_overlap = y1 < y2 + h2 && y1 + h1 > y2;

    x_overlap && y_overlap
}

// Check for overlaps between sibling boxes
fn check_sibling_overlaps(
    boxes: &[Box],
    layout_map: &HashMap<String, (i32, i32, i32, i32)>,
    _parent_bounds: Option<(i32, i32, i32, i32)>,
) -> Result<(), String> {
    // Get all boxes with layout information
    let mut box_bounds: Vec<(String, i32, i32, i32, i32)> = Vec::new();

    for box_item in boxes {
        if let Some(ref id) = box_item.id {
            if let Some(&(x, y, width, height)) = layout_map.get(id) {
                box_bounds.push((id.clone(), x, y, width, height));
            }
        }
    }

    // Check each pair of sibling boxes for overlap
    for i in 0..box_bounds.len() {
        for j in (i + 1)..box_bounds.len() {
            let (id1, x1, y1, w1, h1) = &box_bounds[i];
            let (id2, x2, y2, w2, h2) = &box_bounds[j];

            if boxes_overlap((*x1, *y1, *w1, *h1), (*x2, *y2, *w2, *h2)) {
                return Err(format!(
                    "Boxes '{}' at ({}, {}) with size ({}, {}) and '{}' at ({}, {}) with size ({}, {}) overlap",
                    id1, x1, y1, w1, h1, id2, x2, y2, w2, h2
                ));
            }
        }
    }

    Ok(())
}

// Check that all boxes are within canvas bounds
fn check_canvas_bounds(
    layout_map: &HashMap<String, (i32, i32, i32, i32)>,
    canvas_width: i32,
    canvas_height: i32,
) -> Result<(), String> {
    for (id, &(x, y, width, height)) in layout_map.iter() {
        // Check if box extends beyond canvas boundaries
        if x < 0 || y < 0 {
            return Err(format!(
                "Box '{}' at ({}, {}) has negative coordinates (canvas starts at 0, 0)",
                id, x, y
            ));
        }

        if x + width > canvas_width {
            return Err(format!(
                "Box '{}' extends beyond canvas width: right edge at {} but canvas width is {}",
                id, x + width, canvas_width
            ));
        }

        if y + height > canvas_height {
            return Err(format!(
                "Box '{}' extends beyond canvas height: bottom edge at {} but canvas height is {}",
                id, y + height, canvas_height
            ));
        }
    }

    Ok(())
}

// Add arrowhead marker definition to SVG
fn add_arrowhead_marker(doc: SvgDocument) -> SvgDocument {
    use svg::node::element::{Marker, Polygon, Definitions};

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
    doc.add(defs)
}

// Render the diagram AST as an SVG
pub fn render_diagram_to_svg(doc: &Document, filename: &str, scale_factor: f64, transparent: bool, background_color: Option<&str>) {
    // Get canvas size from layout or use defaults
    let (width, height) = doc.layout.canvas_size.unwrap_or((800, 600));

    // Scale the rendered size while keeping coordinates at original scale
    let display_width = (width as f64 * scale_factor) as i32;
    let display_height = (height as f64 * scale_factor) as i32;

    let mut svg_doc = SvgDocument::new()
        .set("viewBox", (0, 0, width, height))  // Keep original coordinate space
        .set("width", display_width)             // Scale display size
        .set("height", display_height);

    // Add background based on options
    if let Some(color) = background_color {
        // Use specified background color (map through color table)
        let bg_color = map_color(color);
        let background = Rectangle::new()
            .set("width", "100%")
            .set("height", "100%")
            .set("fill", bg_color);
        svg_doc = svg_doc.add(background);
    } else if !transparent {
        // Use white background if not transparent and no color specified
        let background = Rectangle::new()
            .set("width", "100%")
            .set("height", "100%")
            .set("fill", "#FFFFFF");
        svg_doc = svg_doc.add(background);
    }
    // Otherwise, leave transparent (no background)

    // Add arrowhead marker definition
    svg_doc = add_arrowhead_marker(svg_doc);

    // Build layout map
    let layout_map = build_layout_map(doc);

    // Validate layout before rendering
    if let Err(e) = validate_layout(doc, &layout_map) {
        eprintln!("Layout validation error: {}", e);
        std::process::exit(1);
    }

    // Collect text elements while rendering boxes
    let mut text_elements = Vec::new();

    // Render all boxes (rectangles only) using layout information
    // Start with zero parent offset (no parent)
    svg_doc = render_boxes_with_layout(&doc.diagram.boxes, &layout_map, svg_doc, &mut text_elements, 0, 0);

    // Build port position map
    let port_map = build_port_map(doc, &layout_map);

    // Render arrows
    svg_doc = render_arrows(&doc.diagram.arrows, &port_map, svg_doc);

    // Render ports
    svg_doc = render_ports(&doc.diagram.ports, &doc.diagram.boxes, &port_map, svg_doc, &mut text_elements);

    // Render all text elements on top (so text is always in front)
    for text_element in text_elements {
        svg_doc = svg_doc.add(text_element);
    }

    // Save to file
    svg::save(filename, &svg_doc).unwrap();
    println!("Saved diagram to: {}", filename);
}

// Recursively render boxes with layout information
// parent_offset_x and parent_offset_y track cumulative offset from stacked parent boxes
fn render_boxes_with_layout(
    boxes: &[Box],
    layout_map: &HashMap<String, (i32, i32, i32, i32)>,
    mut doc: SvgDocument,
    text_elements: &mut Vec<svg::node::element::Text>,
    parent_offset_x: i32,
    parent_offset_y: i32,
) -> SvgDocument {
    for box_item in boxes {
        doc = render_box_with_layout(box_item, layout_map, doc, text_elements, parent_offset_x, parent_offset_y);
    }
    doc
}

// Render a single box using layout information
// Children are rendered AFTER parents to ensure they appear in front (higher z-index in SVG)
// Text elements are collected and rendered later on top of all boxes
// parent_offset_x and parent_offset_y track cumulative offset from stacked parent boxes
fn render_box_with_layout(
    box_item: &Box,
    layout_map: &HashMap<String, (i32, i32, i32, i32)>,
    mut doc: SvgDocument,
    text_elements: &mut Vec<svg::node::element::Text>,
    parent_offset_x: i32,
    parent_offset_y: i32,
) -> SvgDocument {
    // Get title from properties (optional)
    let title = box_item.properties.iter()
        .find_map(|p| if let Property::Title(t) = p { Some(t.clone()) } else { None });

    // Check if title should be rendered vertically
    let is_vertical = box_item.properties.iter()
        .any(|p| matches!(p, Property::Vertical));

    // Get stacked count from properties
    let stacked_count = box_item.properties.iter()
        .find_map(|p| if let Property::Stacked(n) = p { Some(*n) } else { None })
        .unwrap_or(0);

    // Get color from properties and map to SVG hex color
    let color_name = box_item.properties.iter()
        .find_map(|p| if let Property::Color(c) = p { Some(c.clone()) } else { None })
        .unwrap_or_else(|| "gray".to_string());
    let svg_color = map_color(&color_name);

    // Calculate contrasting text color based on background
    let text_color = get_contrast_text_color(svg_color);

    // Render parent box first (so it appears behind children)
    if let Some(ref id) = box_item.id {
        if let Some(&(x, y, width, height)) = layout_map.get(id) {
            // Apply parent offset to this box's position
            let x = x + parent_offset_x;
            let y = y + parent_offset_y;

            // Draw stacked rectangles behind the main box (if stacked > 0)
            // Background boxes stay at layout position, main box shifts down and right
            if stacked_count > 0 {
                let stack_stagger = 12; // Offset amount in pixels for each stacked box

                // Draw background boxes from back to front
                // Each is offset from the layout position
                for i in (1..=stacked_count).rev() {
                    let offset_x = (stacked_count - i) * stack_stagger;
                    let offset_y = (stacked_count - i) * stack_stagger;

                    let stacked_rect = Rectangle::new()
                        .set("x", x + offset_x)
                        .set("y", y + offset_y)
                        .set("width", width)
                        .set("height", height)
                        .set("fill", svg_color)
                        .set("stroke", "#333")
                        .set("stroke-width", 2)
                        .set("rx", 5);
                    doc = doc.add(stacked_rect);
                }
            }

            // Draw main rectangle for this box (on top of stacked rectangles)
            // Main box is offset down and right by stacked_count * stack_stagger
            let main_offset = if stacked_count > 0 { stacked_count * 12 } else { 0 };
            let rect = Rectangle::new()
                .set("x", x + main_offset)
                .set("y", y + main_offset)
                .set("width", width)
                .set("height", height)
                .set("fill", svg_color)
                .set("stroke", "#333")
                .set("stroke-width", 2)
                .set("rx", 5);
            doc = doc.add(rect);

            // Collect title text element (to be rendered later on top of all boxes)
            // Use contrasting color for readability
            // Text is positioned on the main box (which may be offset if stacked)
            if let Some(title_text) = title {
                let padding = 8; // Padding from edges
                let font_size = 14;

                // Text goes on the main box, which is offset if stacked
                let text_base_x = x + main_offset;
                let text_base_y = y + main_offset;

                if is_vertical {
                    // Vertical text: rotate 90 degrees counter-clockwise around upper left corner,
                    // then shift down in screen coordinates (positive Y direction)

                    // BEFORE rotation: calculate text width
                    let char_count = title_text.chars().count();
                    let estimated_text_width = (char_count as f64 * font_size as f64 * 0.6) as i32;

                    // Upper left corner of the box (with padding)
                    let corner_x = text_base_x + padding;
                    let corner_y = text_base_y + padding;

                    // Shift down in screen coordinates by text width + 2 * padding
                    let translate_y = estimated_text_width + 2 * padding;

                    // Adjust the rotation center down by translate_y in screen coordinates
                    let rotated_corner_x = corner_x;
                    let rotated_corner_y = corner_y + translate_y;

                    // Text position also shifted down (baseline is font_size below the corner)
                    let text_x = corner_x;
                    let text_y = corner_y + font_size + translate_y;

                    // Transform: rotate around the shifted upper left corner
                    let transform = format!("rotate(-90 {} {})",
                                          rotated_corner_x, rotated_corner_y);

                    // Add shadow text (rendered first, behind the main text)
                    let shadow = Text::new(&title_text)
                        .set("x", text_x + 1)
                        .set("y", text_y + 1)
                        .set("text-anchor", "start")
                        .set("font-size", font_size)
                        .set("fill", "rgba(0, 0, 0, 0.3)")
                        .set("opacity", "0.5")
                        .set("transform", transform.clone());
                    text_elements.push(shadow);

                    // Add main text
                    let text = Text::new(&title_text)
                        .set("x", text_x)
                        .set("y", text_y)
                        .set("text-anchor", "start")
                        .set("font-size", font_size)
                        .set("fill", text_color)
                        .set("transform", transform);
                    text_elements.push(text);
                } else {
                    // Horizontal text: positioned in upper left

                    // Add shadow text (rendered first, behind the main text)
                    let shadow = Text::new(&title_text)
                        .set("x", text_base_x + padding + 1)
                        .set("y", text_base_y + padding + font_size + 1)
                        .set("text-anchor", "start")
                        .set("font-size", font_size)
                        .set("fill", "rgba(0, 0, 0, 0.3)")
                        .set("opacity", "0.5");
                    text_elements.push(shadow);

                    // Add main text
                    let text = Text::new(&title_text)
                        .set("x", text_base_x + padding)
                        .set("y", text_base_y + padding + font_size) // Add font size for baseline
                        .set("text-anchor", "start")
                        .set("font-size", font_size)
                        .set("fill", text_color);
                    text_elements.push(text);
                }
            }
            // Calculate cumulative offset for children
            // Children should be offset by parent's offset plus this box's main_offset
            let child_offset_x = parent_offset_x + main_offset;
            let child_offset_y = parent_offset_y + main_offset;

            // Render children AFTER parent (so they appear in front with higher z-index)
            // Pass the cumulative offset to children
            doc = render_boxes_with_layout(&box_item.children, layout_map, doc, text_elements, child_offset_x, child_offset_y);
        } else {
            // No layout found for this identifier
            println!("Warning: No layout found for box with id '{}'", id);

            // Still render children with current parent offset
            doc = render_boxes_with_layout(&box_item.children, layout_map, doc, text_elements, parent_offset_x, parent_offset_y);
        }
    } else {
        // Box has no identifier
        let title_str = title.as_deref().unwrap_or("(no title)");
        println!("Warning: Box '{}' has no identifier, skipping layout", title_str);

        // Still render children with current parent offset
        doc = render_boxes_with_layout(&box_item.children, layout_map, doc, text_elements, parent_offset_x, parent_offset_y);
    }

    doc
}

// Build a map of port positions
// Returns HashMap<port_id, (x, y)>
fn build_port_map(doc: &Document, layout_map: &HashMap<String, (i32, i32, i32, i32)>) -> HashMap<String, (i32, i32)> {
    let mut port_map = HashMap::new();

    // Process top-level ports
    for port in &doc.diagram.ports {
        if let Some(ref id) = port.id {
            let pos = calculate_port_position(port, id, &doc.layout.items, None, layout_map);
            if let Some((x, y)) = pos {
                port_map.insert(id.clone(), (x, y));
            }
        }
    }

    // Process ports inside boxes
    for box_item in &doc.diagram.boxes {
        collect_box_ports(box_item, &doc.layout.items, layout_map, &mut port_map);
    }

    port_map
}

// Recursively collect ports from boxes
fn collect_box_ports(
    box_item: &Box,
    layout_items: &[ast::LayoutItem],
    layout_map: &HashMap<String, (i32, i32, i32, i32)>,
    port_map: &mut HashMap<String, (i32, i32)>,
) {
    let parent_bounds = if let Some(ref box_id) = box_item.id {
        layout_map.get(box_id).copied()
    } else {
        None
    };

    for port in &box_item.ports {
        if let Some(ref id) = port.id {
            let pos = calculate_port_position(port, id, layout_items, parent_bounds, layout_map);
            if let Some((x, y)) = pos {
                port_map.insert(id.clone(), (x, y));
            }
        }
    }

    // Recurse into children
    for child in &box_item.children {
        collect_box_ports(child, layout_items, layout_map, port_map);
    }
}

// Calculate port position based on layout properties
fn calculate_port_position(
    port: &Port,
    port_id: &str,
    layout_items: &[ast::LayoutItem],
    parent_bounds: Option<(i32, i32, i32, i32)>,
    _layout_map: &HashMap<String, (i32, i32, i32, i32)>,
) -> Option<(i32, i32)> {
    // Find layout for this port
    let layout = layout_items.iter().find(|item| item.name == port_id)?;

    // Check if it has a pos property (absolute position)
    for prop in &layout.properties {
        if let LayoutProperty::Pos(x, y) = prop {
            return Some((*x, *y));
        }
    }

    // Check if it has an interp property (interpolated on parent side)
    for prop in &layout.properties {
        if let LayoutProperty::Interp(percentage) = prop {
            // Find which side from port properties
            let side = port.properties.iter()
                .find_map(|p| if let PortProperty::Side(s) = p { Some(s.as_str()) } else { None })
                .unwrap_or("right");

            if let Some((px, py, pw, ph)) = parent_bounds {
                let interp_factor = (*percentage as f64) / 100.0;

                return Some(match side {
                    "left" => (px, py + (ph as f64 * interp_factor) as i32),
                    "right" => (px + pw, py + (ph as f64 * interp_factor) as i32),
                    "top" => (px + (pw as f64 * interp_factor) as i32, py),
                    "bottom" => (px + (pw as f64 * interp_factor) as i32, py + ph),
                    _ => (px + pw, py + (ph as f64 * interp_factor) as i32), // default to right
                });
            }
        }
    }

    None
}

// Render all ports
fn render_ports(
    ports: &[Port],
    boxes: &[Box],
    port_map: &HashMap<String, (i32, i32)>,
    mut doc: SvgDocument,
    text_elements: &mut Vec<Text>,
) -> SvgDocument {
    // Render top-level ports
    for port in ports {
        if let Some(ref id) = port.id {
            if let Some(&(x, y)) = port_map.get(id) {
                doc = render_single_port(port, x, y, doc, text_elements);
            }
        }
    }

    // Render ports in boxes
    for box_item in boxes {
        doc = render_box_ports(box_item, port_map, doc, text_elements);
    }

    doc
}

// Recursively render ports in boxes
fn render_box_ports(
    box_item: &Box,
    port_map: &HashMap<String, (i32, i32)>,
    mut doc: SvgDocument,
    text_elements: &mut Vec<Text>,
) -> SvgDocument {
    for port in &box_item.ports {
        if let Some(ref id) = port.id {
            if let Some(&(x, y)) = port_map.get(id) {
                doc = render_single_port(port, x, y, doc, text_elements);
            }
        }
    }

    for child in &box_item.children {
        doc = render_box_ports(child, port_map, doc, text_elements);
    }

    doc
}

// Render a single port as a circle with an X through it
fn render_single_port(
    port: &Port,
    x: i32,
    y: i32,
    mut doc: SvgDocument,
    text_elements: &mut Vec<Text>,
) -> SvgDocument {
    let radius = 8;

    // Draw circle
    let circle = Circle::new()
        .set("cx", x)
        .set("cy", y)
        .set("r", radius)
        .set("fill", "white")
        .set("stroke", "#333")
        .set("stroke-width", 2);
    doc = doc.add(circle);

    // Draw X through it
    let line1 = Line::new()
        .set("x1", x - radius / 2)
        .set("y1", y - radius / 2)
        .set("x2", x + radius / 2)
        .set("y2", y + radius / 2)
        .set("stroke", "#333")
        .set("stroke-width", 2);
    doc = doc.add(line1);

    let line2 = Line::new()
        .set("x1", x - radius / 2)
        .set("y1", y + radius / 2)
        .set("x2", x + radius / 2)
        .set("y2", y - radius / 2)
        .set("stroke", "#333")
        .set("stroke-width", 2);
    doc = doc.add(line2);

    // Add label if present
    if let Some(title) = port.properties.iter()
        .find_map(|p| if let PortProperty::Title(t) = p { Some(t) } else { None }) {
        let text = Text::new(title)
            .set("x", x + radius + 5)
            .set("y", y + 4)
            .set("font-family", "Arial, sans-serif")
            .set("font-size", 14)
            .set("fill", "#333");
        text_elements.push(text);
    }

    doc
}

// Render all arrows as orthogonal (Manhattan-style) paths
fn render_arrows(
    arrows: &[Arrow],
    port_map: &HashMap<String, (i32, i32)>,
    mut doc: SvgDocument,
) -> SvgDocument {
    for arrow in arrows {
        if let (Some(&(x1, y1)), Some(&(x2, y2))) = (port_map.get(&arrow.from), port_map.get(&arrow.to)) {
            // Create orthogonal path
            let path_data = create_orthogonal_path(x1, y1, x2, y2);

            let path = Path::new()
                .set("d", path_data)
                .set("stroke", "#333")
                .set("stroke-width", 2)
                .set("fill", "none")
                .set("marker-end", "url(#arrowhead)");
            doc = doc.add(path);
        }
    }

    doc
}

// Create an orthogonal path from (x1, y1) to (x2, y2)
// The path goes horizontally first, then vertically, then horizontally again
fn create_orthogonal_path(x1: i32, y1: i32, x2: i32, y2: i32) -> Data {
    let mut data = Data::new().move_to((x1, y1));

    // Calculate midpoint for the vertical segment
    let mid_x = (x1 + x2) / 2;

    // Go horizontally to midpoint
    data = data.line_to((mid_x, y1));

    // Go vertically to destination y
    data = data.line_to((mid_x, y2));

    // Go horizontally to destination
    data = data.line_to((x2, y2));

    data
}
