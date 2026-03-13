// Include the generated parser module
#[macro_use] extern crate lalrpop_util;

use svg::Document as SvgDocument;
use svg::node::element::{Text, Rectangle};
use std::collections::HashMap;

pub mod ast;
use ast::{Document, Box, Property, LayoutProperty};

lalrpop_mod!(pub grammar); // synthesized by LALRPOP

// Build a layout map from the layout section
fn build_layout_map(doc: &Document) -> HashMap<String, (i32, i32, i32, i32)> {
    let mut layout_map = HashMap::new();

    for item in &doc.layout.items {
        let mut pos = (0, 0);
        let mut size = (100, 50); // default size

        for prop in &item.properties {
            match prop {
                LayoutProperty::Pos(x, y) => pos = (*x, *y),
                LayoutProperty::Size(w, h) => size = (*w, *h),
            }
        }

        // Store as (x, y, width, height)
        layout_map.insert(item.name.clone(), (pos.0, pos.1, size.0, size.1));
    }

    layout_map
}

// Map .dia color names to SVG hex color codes
// Colors chosen to match reference.png - muted, professional palette
fn map_color(color_name: &str) -> &str {
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

// Render the diagram AST as an SVG
pub fn render_diagram_to_svg(doc: &Document, filename: &str, scale_factor: f64, transparent: bool) {
    // Get canvas size from layout or use defaults
    let (width, height) = doc.layout.canvas_size.unwrap_or((800, 600));

    // Scale the rendered size while keeping coordinates at original scale
    let display_width = (width as f64 * scale_factor) as i32;
    let display_height = (height as f64 * scale_factor) as i32;

    let mut svg_doc = SvgDocument::new()
        .set("viewBox", (0, 0, width, height))  // Keep original coordinate space
        .set("width", display_width)             // Scale display size
        .set("height", display_height);

    // Add background if not transparent
    if !transparent {
        let background = Rectangle::new()
            .set("width", "100%")
            .set("height", "100%")
            .set("fill", "#FFFFFF");
        svg_doc = svg_doc.add(background);
    }

    // Build layout map
    let layout_map = build_layout_map(doc);

    // Collect text elements while rendering boxes
    let mut text_elements = Vec::new();

    // Render all boxes (rectangles only) using layout information
    svg_doc = render_boxes_with_layout(&doc.diagram.boxes, &layout_map, svg_doc, &mut text_elements);

    // Render all text elements on top (so text is always in front)
    for text_element in text_elements {
        svg_doc = svg_doc.add(text_element);
    }

    // Save to file
    svg::save(filename, &svg_doc).unwrap();
    println!("Saved diagram to: {}", filename);
}

// Recursively render boxes with layout information
fn render_boxes_with_layout(
    boxes: &[Box],
    layout_map: &HashMap<String, (i32, i32, i32, i32)>,
    mut doc: SvgDocument,
    text_elements: &mut Vec<svg::node::element::Text>,
) -> SvgDocument {
    for box_item in boxes {
        doc = render_box_with_layout(box_item, layout_map, doc, text_elements);
    }
    doc
}

// Render a single box using layout information
// Children are rendered AFTER parents to ensure they appear in front (higher z-index in SVG)
// Text elements are collected and rendered later on top of all boxes
fn render_box_with_layout(
    box_item: &Box,
    layout_map: &HashMap<String, (i32, i32, i32, i32)>,
    mut doc: SvgDocument,
    text_elements: &mut Vec<svg::node::element::Text>,
) -> SvgDocument {
    // Get title from properties (optional)
    let title = box_item.properties.iter()
        .find_map(|p| if let Property::Title(t) = p { Some(t.clone()) } else { None });

    // Check if title should be rendered vertically
    let is_vertical = box_item.properties.iter()
        .any(|p| matches!(p, Property::Vertical));

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
            // Draw rectangle for this box
            let rect = Rectangle::new()
                .set("x", x)
                .set("y", y)
                .set("width", width)
                .set("height", height)
                .set("fill", svg_color)
                .set("stroke", "#333")
                .set("stroke-width", 2)
                .set("rx", 5);
            doc = doc.add(rect);

            // Collect title text element (to be rendered later on top of all boxes)
            // Use contrasting color for readability
            if let Some(title_text) = title {
                let padding = 8; // Padding from edges
                let font_size = 14;

                if is_vertical {
                    // Vertical text: rotated 90 degrees counter-clockwise
                    // For vertical text in upper left corner:
                    // 1. Start at upper left corner (x + padding, y + padding)
                    // 2. Rotate -90 degrees makes horizontal text go downward
                    // 3. After rotation, coordinate system is rotated, so:
                    //    - To move DOWN on screen (positive Y), we translate in negative X (after rotation)
                    //    - We need to shift down by font_size to account for baseline
                    let text_x = x + padding;
                    let text_y = y + padding;

                    // Use transform with translate and rotate
                    // Translate to upper left, then rotate around origin, then translate DOWN on screen
                    // After -90 rotation, moving down on screen means translating in negative X
                    let transform = format!("translate({} {}) rotate(-90) translate({} 0)",
                                          text_x, text_y, -font_size);

                    // Add shadow text (rendered first, behind the main text)
                    let shadow = Text::new(&title_text)
                        .set("x", 1)
                        .set("y", 1)
                        .set("text-anchor", "start")
                        .set("font-size", font_size)
                        .set("fill", "rgba(0, 0, 0, 0.3)")
                        .set("opacity", "0.5")
                        .set("transform", transform.clone());
                    text_elements.push(shadow);

                    // Add main text
                    let text = Text::new(&title_text)
                        .set("x", 0)
                        .set("y", 0)
                        .set("text-anchor", "start")
                        .set("font-size", font_size)
                        .set("fill", text_color)
                        .set("transform", transform);
                    text_elements.push(text);
                } else {
                    // Horizontal text: positioned in upper left

                    // Add shadow text (rendered first, behind the main text)
                    let shadow = Text::new(&title_text)
                        .set("x", x + padding + 1)
                        .set("y", y + padding + font_size + 1)
                        .set("text-anchor", "start")
                        .set("font-size", font_size)
                        .set("fill", "rgba(0, 0, 0, 0.3)")
                        .set("opacity", "0.5");
                    text_elements.push(shadow);

                    // Add main text
                    let text = Text::new(&title_text)
                        .set("x", x + padding)
                        .set("y", y + padding + font_size) // Add font size for baseline
                        .set("text-anchor", "start")
                        .set("font-size", font_size)
                        .set("fill", text_color);
                    text_elements.push(text);
                }
            }
        } else {
            // No layout found for this identifier
            println!("Warning: No layout found for box with id '{}'", id);
        }
    } else {
        // Box has no identifier
        let title_str = title.as_deref().unwrap_or("(no title)");
        println!("Warning: Box '{}' has no identifier, skipping layout", title_str);
    }

    // Render children AFTER parent (so they appear in front with higher z-index)
    doc = render_boxes_with_layout(&box_item.children, layout_map, doc, text_elements);

    doc
}
