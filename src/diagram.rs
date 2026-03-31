// Diagram representation with absolute coordinates

pub mod debug;

use crate::{ast, elaboration};

// Re-export color types for backward compatibility
pub use crate::color::{RgbColor, contrast};

// Minimum font size as a fraction of the base font size
const MIN_FONTSIZE: f64 = 0.7;

// Default base font size
const DEFAULT_FONT_SIZE: usize = 18;

// Margin around the top-level box (in pixels)
// Should be large enough to fit the title font (1.5x base font size) plus padding
const TOP_LEVEL_MARGIN: usize = (DEFAULT_FONT_SIZE as f64 * 1.5) as usize + 20;

/// A diagram containing positioned boxes with absolute coordinates
#[derive(Debug)]
pub struct Diagram {
    pub boxes: Vec<DiagramBox>,
    pub ports: Vec<DiagramPort>,
    pub arrows: Vec<DiagramArrow>,
    pub routed_paths: Vec<Vec<(f64, f64)>>, // Routed arrow paths in pixel coordinates
    pub title: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    /// Absolute position in the diagram coordinate space
    pub pos : (f64, f64),
    /// Absolute size (width, height) in the diagram coordinate space
    pub size : (f64, f64),
}

impl Rect {
    /// Create a new Rect
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Rect {
            pos: (x, y),
            size: (width, height),
        }
    }

    /// Get x coordinate
    pub fn x(&self) -> f64 {
        self.pos.0
    }

    /// Get y coordinate
    pub fn y(&self) -> f64 {
        self.pos.1
    }

    /// Get width
    pub fn width(&self) -> f64 {
        self.size.0
    }

    /// Get height
    pub fn height(&self) -> f64 {
        self.size.1
    }

    /// Get right edge x coordinate
    pub fn right(&self) -> f64 {
        self.pos.0 + self.size.0
    }

    /// Get bottom edge y coordinate
    pub fn bottom(&self) -> f64 {
        self.pos.1 + self.size.1
    }

    /// Scale the rectangle by a factor of s, centered at the center of the box
    ///
    /// # Arguments
    /// * `s` - The scaling factor (e.g., 2.0 doubles the size, 0.5 halves it)
    ///
    /// # Returns
    /// A new Rect that is scaled by the factor s, with the same center point
    pub fn scale_at_center(&self, s: f64) -> Rect {
        // Calculate current center
        let center_x = self.pos.0 + self.size.0 / 2.0;
        let center_y = self.pos.1 + self.size.1 / 2.0;

        // Calculate new size
        let new_width = self.size.0 * s;
        let new_height = self.size.1 * s;

        // Calculate new position to maintain the center
        let new_x = center_x - new_width / 2.0;
        let new_y = center_y - new_height / 2.0;

        Rect::new(new_x, new_y, new_width, new_height)
    }
}

/// A port in the diagram with absolute position
#[derive(Debug)]
pub struct DiagramPort {
    pub name: String,
    pub pos: (f64, f64), // Absolute position
    pub label: Option<String>, // Optional label text
    pub parent_rect: Rect,
}

/// An arrow in the diagram connecting two ports
#[derive(Debug)]
pub struct DiagramArrow {
    pub from: String,
    pub to: String,
}

/// A box in the diagram with absolute position and size
#[derive(Debug, Clone)]
pub struct DiagramBox {
    pub rect: Rect,

    pub id: Option<String>,
    pub title: Option<String>,
    pub color: Option<String>,
    /// Font scale factor based on width relative to parent
    pub font_scale: f64,
    /// Whether this box has child boxes
    pub has_children: bool,
    /// Border style: solid, none, dotted, or dashed
    pub border_style: Option<String>,
    /// Horizontal scaling factor relative to top box (ratio: box width / top box width)
    pub horizontal_scaling: f64,
    /// Vertical scaling factor relative to top box (ratio: box height / top box height)
    pub vertical_scaling: f64,
    /// Whether to show debug grid overlay
    pub debug: bool,
    /// Grid dimensions (rows, cols) for debug overlay
    pub grid: (usize, usize),
    /// Name of the box definition (None for inline boxes)
    pub def_name: Option<String>,
    /// Line number where the box was defined
    pub line_number: Option<usize>,
}

impl DiagramBox {
    /// Returns the average scaling factor (average of horizontal and vertical scaling)
    pub fn scaling(&self) -> f64 {
        (self.horizontal_scaling + self.vertical_scaling) / 2.0
    }

    /// Returns the rectangle representing the box's border
    pub fn border(&self) -> Rect {
        self.rect.scale_at_center(0.90)
    }

    /// Returns the rectangle representing the box's border
    pub fn grid(&self) -> Rect {
        self.rect.scale_at_center(0.80)
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
        crate::svg::render_to_svg(self, filename, width, height, font_size, debug)
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
    let mut boxes = Vec::new();
    let mut ports = Vec::new();
    let mut arrows = Vec::new();

    // Calculate absolute positions for all boxes
    // Add margin around the top-level box
    let (canvas_width, canvas_height) = elab_diagram.size;
    let margin = TOP_LEVEL_MARGIN as f64;

    // The top-level box starts at (margin, margin) and has reduced size
    let top_x = margin;
    let top_y = margin;
    let top_width = (canvas_width as f64) - (2.0 * margin);
    let top_height = (canvas_height as f64) - (2.0 * margin);

    // Store top box dimensions for scaling calculations
    let top_box_width = top_width;
    let top_box_height = top_height;

    // Process the top-level box
    // Top box always uses center alignment
    flatten_boxes(
        &elab_diagram.top,
        None, // Top-level box has no ID
        None, // Top-level box has no parent
        canvas_width as f64, // canvas width for font scaling
        top_box_width,
        top_box_height,
        &ast::Alignment::Center,
        &mut boxes,
        &mut ports,
    );

    // Collect all arrows from the top-level box
    collect_arrows(&elab_diagram.top, &mut arrows);

    // Collect routed paths and convert to pixel coordinates
    let mut routed_paths = Vec::new();
    collect_routed_paths(
        &elab_diagram.top,
        top_x,
        top_y,
        top_width,
        top_height,
        &mut routed_paths,
    );

    Diagram {
        boxes,
        ports,
        arrows,
        routed_paths,
        title: elab_diagram.title.clone(),
        color: Some(elab_diagram.color.clone()),
    }
}

/// Recursively collect and convert routed paths from fractional to pixel coordinates
fn collect_routed_paths(
    box_def: &elaboration::BoxInst,
    parent_x: f64,
    parent_y: f64,
    parent_width: f64,
    parent_height: f64,
    output: &mut Vec<Vec<(f64, f64)>>,
) {
    let (grid_height, grid_width) = box_def.grid;

    // Calculate padding if this box has a title and children
    let (padding_top, padding_left, padding_right, padding_bottom) = if box_def.title.is_some() && !box_def.boxes.is_empty() {
        let min_dimension = parent_width.min(parent_height);
        let padding = (min_dimension / 20.0).max(2.0).min(15.0);
        let margin_scale = box_def.margin.unwrap_or(1.0);
        (
            padding * margin_scale,
            padding * margin_scale,
            padding * margin_scale,
            padding * margin_scale,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };

    // Calculate available space after padding
    let available_width = parent_width - (padding_left + padding_right);
    let available_height = parent_height - (padding_top + padding_bottom);

    // Convert routed paths from this box
    for path in &box_def.routed_arrow_paths {
        let pixel_path: Vec<(f64, f64)> = path.iter().map(|(row, col)| {
            // Scale fractional coordinates to pixel coordinates
            let frac_y = row / grid_height as f64;
            let frac_x = col / grid_width as f64;

            // Map to available space, accounting for padding
            let abs_x = parent_x + padding_left + (frac_x * available_width);
            let abs_y = parent_y + padding_top + (frac_y * available_height);

            (abs_x, abs_y)
        }).collect();
        output.push(pixel_path);
    }

    // Recursively collect from child boxes
    for child_box in &box_def.boxes {
        // Calculate child box's absolute position and size
        let (child_row, child_col) = child_box.pos;
        let (child_height, child_width) = child_box.dim;

        let cell_width = available_width / grid_width as f64;
        let cell_height = available_height / grid_height as f64;

        let abs_x = parent_x + padding_left + (child_col as f64 * cell_width);
        let abs_y = parent_y + padding_top + (child_row as f64 * cell_height);

        let box_width = cell_width * child_width as f64;
        let box_height = cell_height * child_height as f64;

        // Apply margin (same logic as flatten_boxes)
        let margin_factor = box_def.margin.unwrap_or(0.1);
        let margin_x = cell_width * margin_factor;
        let margin_y = cell_height * margin_factor;

        let child_x = abs_x + margin_x;
        let child_y = abs_y + margin_y;
        let child_pixel_width = box_width - (2.0 * margin_x);
        let child_pixel_height = box_height - (2.0 * margin_y);

        collect_routed_paths(
            &child_box.def,
            child_x,
            child_y,
            child_pixel_width,
            child_pixel_height,
            output,
        );
    }
}

/// Recursively flatten hierarchical boxes into absolute-positioned boxes
fn flatten_boxes(
    box_def: &elaboration::BoxInst,
    box_id: Option<&str>,
    parent: Option<&DiagramBox>,
    canvas_width: f64,
    top_box_width: f64,
    top_box_height: f64,
    alignment: &ast::Alignment,
    output: &mut Vec<DiagramBox>,
    ports_output: &mut Vec<DiagramPort>,
) {
    // Get the parent grid rectangle - for top-level, use the full canvas dimensions
    let parent_grid = if let Some(p) = parent {
        p.grid()
    } else {
        // Top-level box: create a rect representing the full canvas with margin
        let margin = TOP_LEVEL_MARGIN as f64;
        let top_x = margin;
        let top_y = margin;
        Rect::new(top_x, top_y, top_box_width, top_box_height)
    };

    // Calculate the natural aspect ratio of this box based on its grid
    let (grid_rows, grid_cols) = box_def.grid;
    let natural_aspect_ratio = grid_rows as f64 / grid_cols as f64;

    // Calculate what the natural width and height would be at the top box scale
    // We use the top box as a reference unit
    let natural_width_at_top_scale = top_box_width / grid_cols as f64 * grid_cols as f64;
    let natural_height_at_top_scale = natural_width_at_top_scale * natural_aspect_ratio;

    // Calculate uniform scaling factor to fit this box in the allocated space
    // Use the minimum scaling to preserve aspect ratio and ensure it fits
    let horizontal_ratio = parent_grid.width() / natural_width_at_top_scale;
    let vertical_ratio = parent_grid.height() / natural_height_at_top_scale;
    let uniform_scaling = horizontal_ratio.min(vertical_ratio);

    // Calculate actual box size based on uniform scaling to preserve aspect ratio
    let actual_width = natural_width_at_top_scale * uniform_scaling;
    let actual_height = natural_height_at_top_scale * uniform_scaling;

    // Calculate the center of the parent grid space
    let parent_center_x = parent_grid.x() + parent_grid.width() / 2.0;
    let parent_center_y = parent_grid.y() + parent_grid.height() / 2.0;

    // Start with the box centered in the grid space
    let centered_x = parent_center_x - actual_width / 2.0;
    let centered_y = parent_center_y - actual_height / 2.0;

    // Apply alignment offset from center
    let (offset_x, offset_y) = match alignment {
        ast::Alignment::Top => (0.0, -(parent_center_y - parent_grid.y() - actual_height / 2.0)),
        ast::Alignment::Right => ((parent_grid.right() - parent_center_x - actual_width / 2.0), 0.0),
        ast::Alignment::Bottom => (0.0, (parent_grid.bottom() - parent_center_y - actual_height / 2.0)),
        ast::Alignment::Left => (-(parent_center_x - parent_grid.x() - actual_width / 2.0), 0.0),
        ast::Alignment::Center => (0.0, 0.0),
    };

    let actual_x = centered_x + offset_x;
    let actual_y = centered_y + offset_y;

    // For legacy purposes, set both horizontal and vertical scaling to the same value
    let horizontal_scaling = uniform_scaling;
    let vertical_scaling = uniform_scaling;

    // First, create the current box itself (if it has a title, color, children, ports, or arrows)
    // Boxes with children, ports, or arrows should always be rendered to show their border
    let _current_box = if box_def.title.is_some()
        || box_def.color.is_some()
        || !box_def.boxes.is_empty()
        || !box_def.ports.is_empty()
        || !box_def.arrows.is_empty() {
        // Linear scaling based on box width relative to canvas
        let width_ratio = parent_grid.width() / canvas_width;
        let width_ratio_clamped = width_ratio.min(1.0).max(0.0);
        // Scale linearly from MIN_FONTSIZE to 1.0 based on width
        let font_scale = MIN_FONTSIZE + (1.0 - MIN_FONTSIZE) * width_ratio_clamped;

        let diagram_box = DiagramBox {
            rect: Rect::new(actual_x, actual_y, actual_width, actual_height),
            id: box_id.map(|s| s.to_string()),
            title: box_def.title.clone(),
            color: box_def.color.clone(),
            font_scale,
            has_children: !box_def.boxes.is_empty(),
            border_style: box_def.border_style.clone(),
            horizontal_scaling,
            vertical_scaling,
            debug: box_def.debug.unwrap_or(false),
            grid: box_def.grid,
            def_name: box_def.def_name.clone(),
            line_number: box_def.line_number,
        };
        output.push(diagram_box.clone());
        Some(diagram_box)
    } else {
        None
    };

    // Use actual box dimensions for child positioning
    let box_width_for_children = actual_width;
    let box_height_for_children = actual_height;
    let box_x_for_children = actual_x;
    let box_y_for_children = actual_y;

    // If this box has a title and children, add padding on all sides for the title
    let (padding_top, padding_left, padding_right, padding_bottom) = if box_def.title.is_some() && !box_def.boxes.is_empty() {
        // Calculate padding based on actual box size (matches border radius calculation)
        let min_dimension = box_width_for_children.min(box_height_for_children);
        let padding = (min_dimension / 20.0).max(2.0).min(15.0);

        // Apply margin scaling if specified
        let margin_scale = box_def.margin.unwrap_or(1.0);
        (
            padding * margin_scale,
            padding * margin_scale,
            padding * margin_scale,
            padding * margin_scale,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };

    // Calculate cell size based on actual box dimensions and grid
    // Subtract padding from available space
    let available_width = box_width_for_children - (padding_left + padding_right);
    let available_height = box_height_for_children - (padding_top + padding_bottom);
    let cell_width = available_width / grid_cols as f64;
    let cell_height = available_height / grid_rows as f64;

    // Process all child boxes
    for child_box in &box_def.boxes {
        let (grid_row, grid_col) = child_box.pos;
        let (span_height, span_width) = child_box.dim;

        // Calculate absolute position
        // Add padding to position to account for the title and side padding
        let abs_x = box_x_for_children + padding_left + (grid_col as f64 * cell_width);
        let abs_y = box_y_for_children + padding_top + (grid_row as f64 * cell_height);

        // Box spans multiple cells based on dim field
        let box_width = cell_width * span_width as f64;
        let box_height = cell_height * span_height as f64;

        // Use margin from box definition, defaulting to 0.1 (10%)
        // Margin is based on cell size (not box size) to ensure uniform margins regardless of dim
        let margin_factor = box_def.margin.unwrap_or(0.1);
        let margin_x = cell_width * margin_factor;
        let margin_y = cell_height * margin_factor;

        let final_x = abs_x + margin_x;
        let final_y = abs_y + margin_y;
        let final_width = box_width - (2.0 * margin_x);
        let final_height = box_height - (2.0 * margin_y);

        // Create a synthetic parent box for the child with the allocated rect (including margins)
        let child_parent = DiagramBox {
            rect: Rect::new(final_x, final_y, final_width, final_height),
            id: None,
            title: None,
            color: None,
            font_scale: 1.0,
            has_children: false,
            border_style: None,
            horizontal_scaling: 1.0,
            vertical_scaling: 1.0,
            debug: false,
            grid: (1, 1),
            def_name: None,
            line_number: None,
        };

        // Recursively process this box and its children
        // Use the synthetic parent with margins applied
        flatten_boxes(
            &child_box.def,
            child_box.id.as_deref(),
            Some(&child_parent),
            canvas_width,
            top_box_width,
            top_box_height,
            &child_box.alignment,
            output,
            ports_output,
        );
    }

    // Process ports for this box
    for port in &box_def.ports {
        // Calculate absolute position based on fractional coordinates
        // Port coordinates are (row, col) where row is y and col is x
        // Fractional coordinates range from (0.0, 0.0) to (grid_height, grid_width)
        // Need to scale by grid dimensions to get fractional position in box
        let (grid_height, grid_width) = box_def.grid;

        // Scale coordinates: divide by grid dimensions to get 0.0-1.0 range
        let frac_y = port.coords.0 / grid_height as f64;
        let frac_x = port.coords.1 / grid_width as f64;

        // Map to actual box dimensions, accounting for padding
        // Ports should be positioned within the available space (after padding)
        let abs_x = box_x_for_children + padding_left + (frac_x * available_width); // col is x
        let abs_y = box_y_for_children + padding_top + (frac_y * available_height); // row is y

        // Create the parent rect for this port
        let parent_rect = Rect::new(box_x_for_children, box_y_for_children, box_width_for_children, box_height_for_children);

        ports_output.push(DiagramPort {
            name: port.name.clone(),
            pos: (abs_x, abs_y),
            label: port.label.clone(),
            parent_rect,
        });
    }
}

/// Recursively collect all arrows from a box and its children
fn collect_arrows(box_def: &elaboration::BoxInst, output: &mut Vec<DiagramArrow>) {
    // Add arrows from this box
    for arrow in &box_def.arrows {
        output.push(DiagramArrow {
            from: arrow.from.clone(),
            to: arrow.to.clone(),
        });
    }

    // Recursively collect from child boxes
    for child_box in &box_def.boxes {
        collect_arrows(&child_box.def, output);
    }
}
