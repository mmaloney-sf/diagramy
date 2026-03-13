// Include the generated parser module
#[macro_use] extern crate lalrpop_util;

use svg::Document as SvgDocument;
use svg::node::element::{Text, Rectangle};
use std::fs;

mod ast;
use ast::{Document, Box, Property, LayoutProperty};
use std::collections::HashMap;

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
fn map_color(color_name: &str) -> &str {
    match color_name {
        "red" => "#E74C3C",
        "blue" => "#3498DB",
        "green" => "#2ECC71",
        "yellow" => "#F1C40F",
        "orange" => "#E67E22",
        "purple" => "#9B59B6",
        "pink" => "#FF69B4",
        "cyan" => "#1ABC9C",
        "magenta" => "#E91E63",
        "lime" => "#8BC34A",
        "teal" => "#009688",
        "indigo" => "#3F51B5",
        "brown" => "#795548",
        "gray" => "#95A5A6",
        "grey" => "#95A5A6",
        "black" => "#2C3E50",
        "white" => "#ECF0F1",
        "navy" => "#34495E",
        "maroon" => "#8E44AD",
        "olive" => "#7F8C8D",
        _ => "#95A5A6", // Default to gray for unknown colors
    }
}

// Render the diagram AST as an SVG
fn render_diagram_to_svg(doc: &Document, filename: &str) {
    // Get canvas size from layout or use defaults
    let (width, height) = doc.layout.canvas_size.unwrap_or((800, 600));

    let mut svg_doc = SvgDocument::new()
        .set("viewBox", (0, 0, width, height))
        .set("width", width)
        .set("height", height);

    // Add background
    let background = Rectangle::new()
        .set("width", "100%")
        .set("height", "100%")
        .set("fill", "#f8f9fa");
    svg_doc = svg_doc.add(background);

    // Build layout map
    let layout_map = build_layout_map(doc);

    // Render all boxes using layout information
    svg_doc = render_boxes_with_layout(&doc.diagram.boxes, &layout_map, svg_doc);

    // Save to file
    svg::save(filename, &svg_doc).unwrap();
    println!("Saved diagram to: {}", filename);
}

// Recursively render boxes with layout information
fn render_boxes_with_layout(
    boxes: &[Box],
    layout_map: &HashMap<String, (i32, i32, i32, i32)>,
    mut doc: SvgDocument,
) -> SvgDocument {
    for box_item in boxes {
        doc = render_box_with_layout(box_item, layout_map, doc);
    }
    doc
}

// Render a single box using layout information
// Children are rendered AFTER parents to ensure they appear in front (higher z-index in SVG)
fn render_box_with_layout(
    box_item: &Box,
    layout_map: &HashMap<String, (i32, i32, i32, i32)>,
    mut doc: SvgDocument,
) -> SvgDocument {
    // Get title from properties
    let title = box_item.properties.iter()
        .find_map(|p| if let Property::Title(t) = p { Some(t.clone()) } else { None })
        .unwrap_or_else(|| "Untitled".to_string());

    // Get color from properties and map to SVG hex color
    let color_name = box_item.properties.iter()
        .find_map(|p| if let Property::Color(c) = p { Some(c.clone()) } else { None })
        .unwrap_or_else(|| "gray".to_string());
    let svg_color = map_color(&color_name);

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

            // Draw title text centered in the box
            let text = Text::new(&title)
                .set("x", x + width / 2)
                .set("y", y + height / 2 + 5)
                .set("text-anchor", "middle")
                .set("font-size", 14)
                .set("fill", "white");
            doc = doc.add(text);
        } else {
            // No layout found for this identifier
            println!("Warning: No layout found for box with id '{}'", id);
        }
    } else {
        // Box has no identifier
        println!("Warning: Box '{}' has no identifier, skipping layout", title);
    }

    // Render children AFTER parent (so they appear in front with higher z-index)
    doc = render_boxes_with_layout(&box_item.children, layout_map, doc);

    doc
}

fn main() {
    // Create build directory if it doesn't exist
    std::fs::create_dir_all("build").expect("Failed to create build directory");

    // Create a parser instance
    let parser = grammar::DocumentParser::new();

    // Read the input file from command line argument
    let input_file = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: diagramy <input.dia>");
        std::process::exit(1);
    });

    // Read the input file
    let input = fs::read_to_string(&input_file)
        .expect(&format!("Failed to read {}", input_file));

    // Generate output filename: extract base name and create .svg in build/
    let output_file = {
        use std::path::Path;
        let path = Path::new(&input_file);
        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("diagram");
        format!("build/{}.svg", stem)
    };

    println!("Diagram Parser\n");
    println!("==============\n");

    match parser.parse(&input) {
        Ok(doc) => {
            println!("Successfully parsed diagram!");
            println!("Debug AST: {:#?}\n", doc);
            render_diagram_to_svg(&doc, &output_file);
        }
        Err(e) => {
            println!("Error parsing diagram: {:?}", e);
        }
    }
}
