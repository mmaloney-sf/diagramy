// Debug SVG generation for arrow routing

use crate::routing::{debug, ArrowRouter};

use super::types::{ArrowPath, BoundingBox, Point};
use svg::node::element::{Line, Rectangle, Text};
use svg::Document as SvgDocument;

impl ArrowRouter {
    /// Generate a debug SVG showing the routing problem and solution (if found)
    pub fn generate_routing_debug_svg(
        &self,
        start: Point,
        end: Point,
        arrow_index: usize,
        path: Option<&ArrowPath>,
    ) {
        // Only generate if debug_dir is set
        if let (Some(debug_dir), Some(box_name)) = (self.debug_dir.as_ref(), self.box_name.as_ref())
        {
            debug::generate_routing_debug_svg(
                start,
                end,
                arrow_index,
                path,
                self.grid_width,
                self.grid_height,
                self.grid_resolution,
                &self.obstacle_boxes,
                debug_dir,
                box_name,
            );
        }
    }
}


/// Generate a debug SVG showing the routing problem and solution (if found)
pub fn generate_routing_debug_svg(
    start: Point,
    end: Point,
    arrow_index: usize,
    path: Option<&ArrowPath>,
    grid_width: u64,
    grid_height: u64,
    grid_resolution: i32,
    bounding_boxes: &[BoundingBox],
    debug_dir: &str,
    box_name: &str,
) {
    // Scale factor to convert integral coordinates to pixels
    // We want 100 pixels per original grid square
    // grid_resolution routable squares per original grid square
    // So: 100 / grid_resolution pixels per routable square
    const PIXELS_PER_ORIGINAL_SQUARE: f64 = 100.0;
    let scale = PIXELS_PER_ORIGINAL_SQUARE / grid_resolution as f64;

    // Calculate SVG dimensions
    let svg_width = (grid_width as f64 * PIXELS_PER_ORIGINAL_SQUARE) as usize;
    let svg_height = (grid_height as f64 * PIXELS_PER_ORIGINAL_SQUARE) as usize;

    let mut svg_doc = SvgDocument::new()
        .set("width", svg_width)
        .set("height", svg_height)
        .set("viewBox", (0, 0, svg_width, svg_height));

    // Draw parent box boundary
    let parent_rect = Rectangle::new()
        .set("x", 0)
        .set("y", 0)
        .set("width", svg_width)
        .set("height", svg_height)
        .set("fill", "none")
        .set("stroke", "#000")
        .set("stroke-width", 2);
    svg_doc = svg_doc.add(parent_rect);

    // Draw grid lines
    // Draw fine grid lines for routable squares and bold lines for original grid squares

    // Vertical grid lines
    for i in 1..(grid_width as i32 * grid_resolution) {
        let x = (i as f64 * scale) as i32;
        let is_original_grid = i % grid_resolution == 0;
        let line = Line::new()
            .set("x1", x)
            .set("y1", 0)
            .set("x2", x)
            .set("y2", svg_height)
            .set("stroke", if is_original_grid { "#888888" } else { "#dddddd" })
            .set("stroke-width", if is_original_grid { 2 } else { 1 });
        svg_doc = svg_doc.add(line);
    }

    // Horizontal grid lines
    for i in 1..(grid_height as i32 * grid_resolution) {
        let y = (i as f64 * scale) as i32;
        let is_original_grid = i % grid_resolution == 0;
        let line = Line::new()
            .set("x1", 0)
            .set("y1", y)
            .set("x2", svg_width)
            .set("y2", y)
            .set("stroke", if is_original_grid { "#888888" } else { "#dddddd" })
            .set("stroke-width", if is_original_grid { 2 } else { 1 });
        svg_doc = svg_doc.add(line);
    }

    // Draw bounding boxes (obstacles)
    for bbox in bounding_boxes {
        let x = (bbox.min.1 as f64 * scale) as i32;
        let y = (bbox.min.0 as f64 * scale) as i32;
        let width = ((bbox.max.1 - bbox.min.1) as f64 * scale) as i32;
        let height = ((bbox.max.0 - bbox.min.0) as f64 * scale) as i32;

        let rect = Rectangle::new()
            .set("x", x)
            .set("y", y)
            .set("width", width)
            .set("height", height)
            .set("fill", "#ffcccc")
            .set("stroke", "#ff0000")
            .set("stroke-width", 2);
        svg_doc = svg_doc.add(rect);
    }

    // Draw start point (fill grid square)
    let start_x = (start.1 as f64 * scale) as i32;
    let start_y = (start.0 as f64 * scale) as i32;
    let start_rect = Rectangle::new()
        .set("x", start_x)
        .set("y", start_y)
        .set("width", scale as i32)
        .set("height", scale as i32)
        .set("fill", "#00ff00")
        .set("stroke", "none");
    svg_doc = svg_doc.add(start_rect);

    // Add "START" label
    let start_label = Text::new("START")
        .set("x", start_x + (scale as i32) / 2)
        .set("y", start_y + (scale as i32) / 2 + 4)
        .set("text-anchor", "middle")
        .set("font-family", "Arial, sans-serif")
        .set("font-size", 8)
        .set("font-weight", "bold")
        .set("fill", "#008800");
    svg_doc = svg_doc.add(start_label);

    // Draw end point (fill grid square)
    let end_x = (end.1 as f64 * scale) as i32;
    let end_y = (end.0 as f64 * scale) as i32;
    let end_rect = Rectangle::new()
        .set("x", end_x)
        .set("y", end_y)
        .set("width", scale as i32)
        .set("height", scale as i32)
        .set("fill", "#0000ff")
        .set("stroke", "none");
    svg_doc = svg_doc.add(end_rect);

    // Add "END" label
    let end_label = Text::new("END")
        .set("x", end_x + (scale as i32) / 2)
        .set("y", end_y + (scale as i32) / 2 + 4)
        .set("text-anchor", "middle")
        .set("font-family", "Arial, sans-serif")
        .set("font-size", 8)
        .set("font-weight", "bold")
        .set("fill", "#000088");
    svg_doc = svg_doc.add(end_label);

    // Draw the routed path if it exists
    if let Some(arrow_path) = path {
        // Fill each grid square in the path with light yellow (transparent)
        for point in &arrow_path.points {
            let px = (point.1 as f64 * scale) as i32;
            let py = (point.0 as f64 * scale) as i32;

            let rect = Rectangle::new()
                .set("x", px)
                .set("y", py)
                .set("width", scale as i32)
                .set("height", scale as i32)
                .set("fill", "#aaaaaa")
                .set("fill-opacity", "0.7")
                .set("stroke", "none");
            svg_doc = svg_doc.add(rect);
        }
    }

    // Save to file
    // Create debug directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(debug_dir) {
        eprintln!(
            "Warning: Failed to create debug directory {}: {}",
            debug_dir, e
        );
        return;
    }

    let filename = format!("{}/routing_{}_{}.svg", debug_dir, box_name, arrow_index);

    if let Err(e) = svg::save(&filename, &svg_doc) {
        eprintln!("Warning: Failed to save routing debug SVG: {}", e);
    } else {
        eprintln!("Routing debug SVG saved to: {}", filename);
    }
}
