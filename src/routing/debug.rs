// Debug SVG generation for arrow routing

use crate::routing::{debug, ArrowRouter};

use super::types::{ArrowPath, BoundingBox, Point};
use svg::node::element::{Circle, Line, Path as SvgPath, Rectangle, Text};
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
    bounding_boxes: &[BoundingBox],
    debug_dir: &str,
    box_name: &str,
) {
    // Scale factor to convert integral coordinates to pixels
    // GRID_RESOLUTION is 0.1, so we need to scale by 10.0 to get back to fractional units
    const SCALE: f64 = 10.0; // 100.0 pixels per unit * 0.1 units per grid cell = 10 pixels per grid cell

    // Calculate SVG dimensions
    let svg_width = (grid_width as f64 * 100.0) as usize;
    let svg_height = (grid_height as f64 * 100.0) as usize;

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

    // Draw grid lines (1x1 squares)
    // Vertical grid lines
    let mut x = SCALE;
    while x < svg_width as f64 {
        let line = Line::new()
            .set("x1", x as i32)
            .set("y1", 0)
            .set("x2", x as i32)
            .set("y2", svg_height)
            .set("stroke", "#cccccc")
            .set("stroke-width", 1);
        svg_doc = svg_doc.add(line);
        x += SCALE;
    }

    // Horizontal grid lines
    let mut y = SCALE;
    while y < svg_height as f64 {
        let line = Line::new()
            .set("x1", 0)
            .set("y1", y as i32)
            .set("x2", svg_width)
            .set("y2", y as i32)
            .set("stroke", "#cccccc")
            .set("stroke-width", 1);
        svg_doc = svg_doc.add(line);
        y += SCALE;
    }

    // Draw bounding boxes (obstacles)
    for bbox in bounding_boxes {
        let x = (bbox.min.1 as f64 * SCALE) as i32;
        let y = (bbox.min.0 as f64 * SCALE) as i32;
        let width = ((bbox.max.1 - bbox.min.1) as f64 * SCALE) as i32;
        let height = ((bbox.max.0 - bbox.min.0) as f64 * SCALE) as i32;

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

    // Draw start point (green circle)
    let start_x = (start.1 as f64 * SCALE) as i32;
    let start_y = (start.0 as f64 * SCALE) as i32;
    let start_circle = Circle::new()
        .set("cx", start_x)
        .set("cy", start_y)
        .set("r", 8)
        .set("fill", "#00ff00")
        .set("stroke", "#008800")
        .set("stroke-width", 2);
    svg_doc = svg_doc.add(start_circle);

    // Add "START" label
    let start_label = Text::new("START")
        .set("x", start_x)
        .set("y", start_y + 20)
        .set("text-anchor", "middle")
        .set("font-family", "Arial, sans-serif")
        .set("font-size", 12)
        .set("font-weight", "bold")
        .set("fill", "#008800");
    svg_doc = svg_doc.add(start_label);

    // Draw end point (blue circle)
    let end_x = (end.1 as f64 * SCALE) as i32;
    let end_y = (end.0 as f64 * SCALE) as i32;
    let end_circle = Circle::new()
        .set("cx", end_x)
        .set("cy", end_y)
        .set("r", 8)
        .set("fill", "#0000ff")
        .set("stroke", "#000088")
        .set("stroke-width", 2);
    svg_doc = svg_doc.add(end_circle);

    // Add "END" label
    let end_label = Text::new("END")
        .set("x", end_x)
        .set("y", end_y + 20)
        .set("text-anchor", "middle")
        .set("font-family", "Arial, sans-serif")
        .set("font-size", 12)
        .set("font-weight", "bold")
        .set("fill", "#000088");
    svg_doc = svg_doc.add(end_label);

    // Draw the routed path if it exists
    if let Some(arrow_path) = path {
        let mut path_data = String::new();

        for (i, point) in arrow_path.points.iter().enumerate() {
            let px = (point.1 as f64 * SCALE) as i32;
            let py = (point.0 as f64 * SCALE) as i32;

            if i == 0 {
                path_data.push_str(&format!("M {} {} ", px, py));
            } else {
                path_data.push_str(&format!("L {} {} ", px, py));
            }
        }

        let path_elem = SvgPath::new()
            .set("d", path_data)
            .set("stroke", "#ff00ff")
            .set("stroke-width", 3)
            .set("fill", "none")
            .set("opacity", 0.7);
        svg_doc = svg_doc.add(path_elem);
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
