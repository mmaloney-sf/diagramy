// Include the generated parser module
#[macro_use] extern crate lalrpop_util;

use svg::Document as SvgDocument;
use svg::node::element::{Text, Rectangle};
use std::fs;

mod ast;
use ast::{Document, Box, Property};

lalrpop_mod!(pub grammar); // synthesized by LALRPOP

// Render the diagram AST as an SVG
fn render_diagram_to_svg(doc: &Document, filename: &str) {
    let width = 800;
    let height = 600;

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

    // For now, just render a simple tree structure
    // This is a placeholder - we'll improve the layout later
    let mut y_pos = 50.0;

    for box_item in &doc.diagram.boxes {
        svg_doc = render_box(box_item, 50.0, y_pos, 0, svg_doc);
        y_pos += 150.0;
    }

    // Save to file
    svg::save(filename, &svg_doc).unwrap();
    println!("Saved diagram to: {}", filename);
}

fn render_box(box_item: &Box, x: f32, y: f32, depth: usize, mut doc: SvgDocument) -> SvgDocument {
    let indent = depth as f32 * 30.0;
    let box_x = x + indent;

    // Get title from properties
    let title = box_item.properties.iter()
        .find_map(|p| if let Property::Title(t) = p { Some(t.clone()) } else { None })
        .unwrap_or_else(|| "Untitled".to_string());

    // Get color from properties
    let color = box_item.properties.iter()
        .find_map(|p| if let Property::Color(c) = p { Some(c.clone()) } else { None })
        .unwrap_or_else(|| "gray".to_string());

    // Draw rectangle
    let rect = Rectangle::new()
        .set("x", box_x)
        .set("y", y)
        .set("width", 150)
        .set("height", 40)
        .set("fill", color)
        .set("stroke", "#333")
        .set("stroke-width", 2)
        .set("rx", 5);
    doc = doc.add(rect);

    // Draw title text
    let text = Text::new(title)
        .set("x", box_x + 75.0)
        .set("y", y + 25.0)
        .set("text-anchor", "middle")
        .set("font-size", 14)
        .set("fill", "white");
    doc = doc.add(text);

    // Render children
    let mut child_y = y + 60.0;
    for child in &box_item.children {
        doc = render_box(child, x, child_y, depth + 1, doc);
        child_y += 60.0;
    }

    doc
}

fn main() {
    // Create build directory if it doesn't exist
    std::fs::create_dir_all("build").expect("Failed to create build directory");

    // Create a parser instance
    let parser = grammar::DocumentParser::new();

    // Read the example file
    let test_file = std::env::args().nth(1).unwrap_or_else(|| "examples/test.dia".to_string());
    let input = fs::read_to_string(&test_file)
        .expect(&format!("Failed to read {}", test_file));

    println!("Diagram Parser\n");
    println!("==============\n");

    match parser.parse(&input) {
        Ok(doc) => {
            println!("Successfully parsed diagram!");
            println!("Debug AST: {:#?}\n", doc);
            render_diagram_to_svg(&doc, "build/diagram.svg");
        }
        Err(e) => {
            println!("Error parsing diagram: {:?}", e);
        }
    }
}
