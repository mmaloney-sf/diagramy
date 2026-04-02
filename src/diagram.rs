// Diagram representation with absolute coordinates

pub mod debug;

use std::sync::Arc;

use crate::rect::Rect;
use crate::elaboration::{self, BoxDef};
use crate::ast;

// Re-export color types for backward compatibility
pub use crate::color::{RgbColor, contrast};

const MARGIN_FACTOR: f64 = 0.05;

/// A diagram containing positioned boxes with absolute coordinates
#[derive(Debug)]
pub struct Diagram {
    pub top: DiagramBox,
    pub debug: bool,
}

#[derive(Debug, Clone)]
pub struct DiagramBox {
    pub boxdef: Arc<BoxDef>,

    pub bounds: Rect,
    pub margin: f64,
    pub padding: f64,
    //pub font_size: f64,

    pub children: Vec<DiagramBox>,
}


#[derive(Debug, Clone)]
pub struct DiagramLabel {
    pub bounds: Rect,
    pub text: String,
}

impl DiagramBox {
    pub fn bounds(&self) -> Rect {
        self.bounds.clone()
    }

    pub fn border_bounds(&self) -> Rect {
        self.bounds.margin(self.margin)
    }
    pub fn grid_bounds(&self) -> Rect {
        self.border_bounds().margin(self.padding)
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
    /// * `debug` - Whether to include debug overlay
    pub fn render_to_svg(&self, filename: &str, width: usize, height: usize, font_size: usize, debug: bool) -> Result<(), String> {
        crate::svg::render_to_svg(self, filename, width, height, font_size, debug || self.debug)
    }

    /// Render the diagram to an SVG string (for WebAssembly)
    ///
    /// # Arguments
    /// * `width` - Width of the SVG canvas
    /// * `height` - Height of the SVG canvas
    /// * `font_size` - Font size for text rendering (default: 18)
    /// * `debug` - Whether to include debug overlay
    #[cfg(all(target_arch = "wasm32", feature = "wasm-bindgen"))]
    pub fn render_to_svg_string(&self, width: usize, height: usize, font_size: usize, debug: bool) -> Result<String, String> {
        crate::svg::render_to_svg_string(self, width, height, font_size, debug)
    }
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

/// Convert an elaboration::Diagram to a diagram::Diagram with absolute coordinates
pub fn from_elaboration(elab_diagram: &elaboration::ElaboratedDiagram) -> Diagram {
    Diagram::from_elaboration(elab_diagram)
}

impl Diagram {
    pub fn from_elaboration(elab_diagram: &elaboration::ElaboratedDiagram) -> Diagram {
        let (canvas_width, canvas_height) = elab_diagram.size;
        let diagram_bounds = Rect::new(0.0, 0.0, canvas_width as f64, canvas_height as f64);

        let mut diagram = Diagram {
            debug: elab_diagram.debug,
            top: DiagramBox {
                bounds: Rect::new(0.0, 0.0, 0.0, 0.0), // Placeholder
                margin: 0.0,
                padding: 0.0,
                boxdef: elab_diagram.top.clone(),
                children: vec![],
            },
        };

        diagram.top = diagram.create_diagram_box(&elab_diagram.top, diagram_bounds);
        diagram
    }

    fn create_diagram_box(&mut self, box_inst: &Arc<elaboration::BoxDef>, bounds: Rect) -> DiagramBox {
        let margin = bounds.width().min(bounds.height()) as f64 * MARGIN_FACTOR;
        let padding = margin;
        // Check if this is a label (has title and border_style "none")
        let is_label = box_inst.title.is_some()
            && box_inst.border_style.as_deref() == Some("none");

        // Calculate child bounds and create child DiagramBoxes
        let border_bounds = bounds.margin(margin);
        let grid_bounds = border_bounds.margin(padding);
        let mut children = Vec::new();

        for child_box in &box_inst.boxes {
            let stretch = false;
            let child_bounds = if stretch {
                let (max_row, max_col) = box_inst.grid;
                let dr = grid_bounds.height() / max_row as f64;
                let dc = grid_bounds.width() / max_col as f64;

                let (child_pos_row, child_pos_col) = child_box.pos;

                let rendered_child_pos_row = (child_pos_row as f64) * dr;
                let rendered_child_pos_col = (child_pos_col as f64) * dc;

                // dim is (height, width) - number of grid cells to span
                let (child_dim_height, child_dim_width) = child_box.dim;
                let rendered_child_dim_height = child_dim_height as f64 * dr;
                let rendered_child_dim_width  = child_dim_width  as f64 * dc;

                let x = grid_bounds.x() + rendered_child_pos_col;
                let y = grid_bounds.y() + rendered_child_pos_row;

                Rect::new(
                    x,
                    y,
                    rendered_child_dim_width,
                    rendered_child_dim_height,
                )
            } else {
                // Calculate the allocated space on the parent's grid
                let (max_row, max_col) = box_inst.grid;
                let dr = grid_bounds.height() / max_row as f64;
                let dc = grid_bounds.width() / max_col as f64;

                let (child_pos_row, child_pos_col) = child_box.pos;
                let (child_dim_height, child_dim_width) = child_box.dim;

                // Allocated dimensions from parent's grid
                let allocated_width = child_dim_width as f64 * dc;
                let allocated_height = child_dim_height as f64 * dr;

                // Get the natural aspect ratio of the child box
                let (child_grid_rows, child_grid_cols) = child_box.def.grid;
                let child_aspect_ratio = child_grid_rows as f64 / child_grid_cols as f64;

                // Calculate natural dimensions at some scale
                let natural_width = 1.0;  // Arbitrary reference
                let natural_height = natural_width * child_aspect_ratio;

                // Find the largest scale factor s that fits in both dimensions
                let scale_x = allocated_width / natural_width;
                let scale_y = allocated_height / natural_height;
                let s = scale_x.min(scale_y);

                // Calculate actual child size
                let actual_width = natural_width * s;
                let actual_height = natural_height * s;

                // Calculate allocated position on parent's grid
                let allocated_x = grid_bounds.x() + (child_pos_col as f64 * dc);
                let allocated_y = grid_bounds.y() + (child_pos_row as f64 * dr);

                // Position the child based on alignment
                let (x, y) = match child_box.alignment {
                    ast::Alignment::Center => {
                        (allocated_x + (allocated_width - actual_width) / 2.0,
                         allocated_y + (allocated_height - actual_height) / 2.0)
                    },
                    ast::Alignment::Top => {
                        (allocated_x + (allocated_width - actual_width) / 2.0,
                         allocated_y)
                    },
                    ast::Alignment::Bottom => {
                        (allocated_x + (allocated_width - actual_width) / 2.0,
                         allocated_y + allocated_height - actual_height)
                    },
                    ast::Alignment::Left => {
                        (allocated_x,
                         allocated_y + (allocated_height - actual_height) / 2.0)
                    },
                    ast::Alignment::Right => {
                        (allocated_x + allocated_width - actual_width,
                         allocated_y + (allocated_height - actual_height) / 2.0)
                    },
                };

                Rect::new(x, y, actual_width, actual_height)
            };

            // Recursively create child DiagramBox
            let child_diagram_box = self.create_diagram_box(&child_box.def, child_bounds);
            children.push(child_diagram_box);
        }

        // Create the DiagramBox with its children
        let diagram_box = DiagramBox {
            bounds,
            margin,
            padding,
            boxdef: box_inst.clone(),
            children,
        };

        /*
        // Process ports - convert grid coordinates to absolute positions
        for port in &box_inst.ports {
            // Port coords are in grid space (0.0 to grid_height/width), not fractional (0.0-1.0)
            // Normalize by dividing by grid dimensions
            let (grid_rows, grid_cols) = box_inst.grid;
            let (port_row, port_col) = port.coords;

            // Normalize to 0.0-1.0 range
            let frac_x = port_col / grid_cols as f64;
            let frac_y = port_row / grid_rows as f64;

            // Ports are positioned relative to the grid bounds (not grid bounds)
            let abs_x = border_bounds.x() + frac_x * border_bounds.width();
            let abs_y = border_bounds.y() + frac_y * border_bounds.height();

            self.elements.push(DiagramElement::Port(DiagramPort {
                name: port.name.clone(),
                pos: (abs_x, abs_y),
                label: port.label.clone(),
            }));
        }
        */

        diagram_box
    }
}
