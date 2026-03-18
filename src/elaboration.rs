use crate::ast;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug)]
pub struct ElaboratedDiagram {
    pub color: String,
    pub size: (usize, usize),
    pub title: Option<String>,
    pub top: Arc<BoxDef>,
}

#[derive(Debug)]
pub struct BoxDef {
    pub grid: (usize, usize),
    pub title: Option<String>,
    pub color: Option<String>,
    pub margin: Option<f64>,
    pub border_style: Option<String>,
    pub boxes: Vec<Box>,
    pub ports: Vec<Port>,
    pub arrows: Vec<Arrow>,
    pub routed_arrow_paths: Vec<Vec<(f64, f64)>>, // Routed paths in fractional coordinates
}

#[derive(Debug, Clone)]
pub struct Port {
    pub name: String,
    pub coords: (f64, f64), // Fractional coordinates
}

#[derive(Debug, Clone)]
pub struct Arrow {
    pub from: String,
    pub to: String,
}

#[derive(Debug)]
pub struct Box {
    pub def: Arc<BoxDef>,
    pub pos: (usize, usize),
    pub dim: (usize, usize), // (height, width) - number of grid cells to span
}

/// Convert an ast::Document into a diagram::Diagram
pub fn from_ast(
    doc: &ast::Document,
    source: &str,
    filename: &str,
    debug_dir: Option<&str>,
) -> Result<ElaboratedDiagram, String> {
    // Extract diagram-level properties
    let mut color = String::from("transparent");
    let mut width: Option<usize> = None;
    let mut title: Option<String> = None;
    let mut top_name: Option<String> = None;

    for prop in &doc.diagram.props {
        match prop {
            ast::Prop::PropIdent { key, value, .. } if key == "color" => {
                color = value.clone();
            }
            ast::Prop::PropIdent { key, value, .. } if key == "top" => {
                top_name = Some(value.clone());
            }
            ast::Prop::PropNumber { key, value, .. } if key == "width" => {
                width = Some(*value as usize);
            }
            ast::Prop::PropString { key, value, .. } if key == "title" => {
                title = Some(value.join("\n"));
            }
            _ => {}
        }
    }

    // Build a map of box definitions for reference lookup
    let mut box_def_map: HashMap<String, &ast::BoxDef> = HashMap::new();
    for box_def in &doc.box_defs {
        box_def_map.insert(box_def.name.clone(), box_def);
    }

    // Find the top box definition based on the following priority:
    // 1. If "top" property is specified in diagram section, use that BoxDef
    // 2. Otherwise, use the first box definition
    // 3. If no box definitions exist, error out
    let top_ast_def = if let Some(ref name) = top_name {
        // top: property was specified, look it up
        box_def_map
            .get(name)
            .copied()
            .ok_or_else(|| format!("{}:0:0: No such box: {}", filename, name))?
    } else {
        // No top: property, use first box
        doc.box_defs.first().ok_or_else(|| {
            format!(
                "{}:0:0: Document must have at least one box definition",
                filename
            )
        })?
    };

    // Convert the top box definition
    let top_box_def = convert_ast_box_body(
        &top_ast_def.body,
        &box_def_map,
        source,
        filename,
        debug_dir,
        "top",
    )?;

    // Calculate size from width and grid aspect ratio
    // grid is now (rows, cols), so aspect_ratio = rows / cols
    let width = width.unwrap_or(800); // default width
    let (grid_rows, grid_cols) = top_box_def.grid;
    let aspect_ratio = grid_rows as f64 / grid_cols as f64;
    let height = (width as f64 * aspect_ratio) as usize;
    let size = (width, height);

    Ok(ElaboratedDiagram {
        color,
        size,
        title,
        top: Arc::new(top_box_def),
    })
}

/// Process an inline box instance (WithBody variant)
fn process_inline_box(
    with_body: &ast::WithBody,
    box_def_map: &HashMap<String, &ast::BoxDef>,
    source: &str,
    filename: &str,
    debug_dir: Option<&str>,
    box_name: &str,
    grid: (usize, usize),
    occupied: &mut HashSet<(i32, i32)>,
    last_pos: &mut (i32, i32),
) -> Result<Box, String> {
    // Determine position (auto-position if coords is None)
    let (row, col) = if let Some(c) = &with_body.coords {
        (c.row, c.col)
    } else {
        match find_next_free_position(
            occupied,
            grid,
            (with_body.dim.height, with_body.dim.width),
            *last_pos,
        ) {
            Some(pos) => pos,
            None => {
                let start = with_body.span.start();
                return Err(format!(
                    "{}:{}:{}: Cannot auto-position box with dim {}x{}. No free space available in {}x{} grid",
                    filename, start.line(), start.col(), with_body.dim.height, with_body.dim.width, grid.0, grid.1
                ));
            }
        }
    };

    // Update last position
    *last_pos = (row, col);

    // Check for overlaps and mark occupied cells (including cells occupied by dim)
    for r in row..(row + with_body.dim.height) {
        for c in col..(col + with_body.dim.width) {
            if occupied.contains(&(r, c)) {
                let start = with_body.span.start();
                return Err(format!(
                    "{}:{}:{}: Box at ({}, {}) with dim {}x{} overlaps with another box at cell ({}, {})",
                    filename, start.line(), start.col(), row, col, with_body.dim.height, with_body.dim.width, r, c
                ));
            }
            occupied.insert((r, c));
        }
    }

    // Recursively convert the nested box body
    let nested_def = convert_ast_box_body(
        &with_body.body,
        box_def_map,
        source,
        filename,
        debug_dir,
        &format!("{}.inline", box_name),
    )?;

    Ok(Box {
        def: Arc::new(nested_def),
        // Convert from 1-based to 0-based indexing
        pos: ((row - 1) as usize, (col - 1) as usize),
        dim: (with_body.dim.height as usize, with_body.dim.width as usize),
    })
}

/// Process a box reference (Reference variant)
fn process_box_reference(
    reference: &ast::Reference,
    box_def_map: &HashMap<String, &ast::BoxDef>,
    source: &str,
    filename: &str,
    debug_dir: Option<&str>,
    grid: (usize, usize),
    occupied: &mut HashSet<(i32, i32)>,
    last_pos: &mut (i32, i32),
) -> Result<Box, String> {
    // Determine position (auto-position if coords is None)
    let (row, col) = if let Some(c) = &reference.coords {
        (c.row, c.col)
    } else {
        match find_next_free_position(
            occupied,
            grid,
            (reference.dim.height, reference.dim.width),
            *last_pos,
        ) {
            Some(pos) => pos,
            None => {
                let start = reference.span.start();
                return Err(format!(
                    "{}:{}:{}: Cannot auto-position box '{}' with dim {}x{}. No free space available in {}x{} grid",
                    filename, start.line(), start.col(), reference.def_name, reference.dim.height, reference.dim.width, grid.0, grid.1
                ));
            }
        }
    };

    // Update last position
    *last_pos = (row, col);

    // Check for overlaps and mark occupied cells (including cells occupied by dim)
    for r in row..(row + reference.dim.height) {
        for c in col..(col + reference.dim.width) {
            if occupied.contains(&(r, c)) {
                let start = reference.span.start();
                return Err(format!(
                    "{}:{}:{}: Box at ({}, {}) with dim {}x{} overlaps with another box at cell ({}, {})",
                    filename, start.line(), start.col(), row, col, reference.dim.height, reference.dim.width, r, c
                ));
            }
            occupied.insert((r, c));
        }
    }

    // Look up the referenced box definition
    let referenced_def = box_def_map.get(&reference.def_name).ok_or_else(|| {
        let start = reference.span.start();
        format!(
            "{}:{}:{}: No such box: {}",
            filename,
            start.line(),
            start.col(),
            reference.def_name
        )
    })?;

    let nested_def = convert_ast_box_body(
        &referenced_def.body,
        box_def_map,
        source,
        filename,
        debug_dir,
        &reference.def_name,
    )?;

    Ok(Box {
        def: Arc::new(nested_def),
        // Convert from 1-based to 0-based indexing
        pos: ((row - 1) as usize, (col - 1) as usize),
        dim: (reference.dim.height as usize, reference.dim.width as usize),
    })
}

/// Extract properties, ports, and arrows from a box body
/// Returns (grid, title, color, margin, border_style, ports, arrows)
fn extract_box_items(
    body: &ast::BoxBody,
) -> (
    (usize, usize),
    Option<String>,
    Option<String>,
    Option<f64>,
    Option<String>,
    Vec<Port>,
    Vec<Arrow>,
) {
    let mut grid = (1, 1); // default grid
    let mut title: Option<String> = None;
    let mut color: Option<String> = None;
    let mut margin: Option<f64> = None;
    let mut border_style: Option<String> = None;
    let mut ports: Vec<Port> = Vec::new();
    let mut arrows: Vec<Arrow> = Vec::new();

    for item in &body.items {
        match item {
            ast::BoxItem::Prop(prop) => match prop {
                ast::Prop::PropDim { key, value, .. } if key == "grid" => {
                    grid = (value.height as usize, value.width as usize);
                }
                ast::Prop::PropString { key, value, .. } if key == "text" => {
                    title = Some(value.join("\n"));
                }
                ast::Prop::PropIdent { key, value, .. } if key == "color" => {
                    color = Some(value.clone());
                }
                ast::Prop::PropIdent { key, value, .. } if key == "borderStyle" => {
                    border_style = Some(value.clone());
                }
                ast::Prop::PropFrac { key, value, .. } if key == "margin" => {
                    margin = Some(*value);
                }
                _ => {}
            },
            ast::BoxItem::Port(port) => {
                ports.push(Port {
                    name: port.name.clone(),
                    coords: (port.coords.row, port.coords.col),
                });
            }
            ast::BoxItem::Arrow(arrow) => {
                arrows.push(Arrow {
                    from: arrow.from.to_string(),
                    to: arrow.to.to_string(),
                });
            }
            _ => {}
        }
    }

    (grid, title, color, margin, border_style, ports, arrows)
}

/// Find the next free grid position that can fit a box with the given dimensions
/// Starts scanning from the position FOLLOWING last_pos
/// Returns Some((row, col)) in 1-based indexing, or None if no position found
fn find_next_free_position(
    occupied: &HashSet<(i32, i32)>,
    grid: (usize, usize),
    dim: (i32, i32),
    last_pos: (i32, i32),
) -> Option<(i32, i32)> {
    let (grid_rows, grid_cols) = grid;
    let (dim_height, dim_width) = dim;
    let (last_row, last_col) = last_pos;

    // Calculate the starting position (next position after last_pos)
    let (start_row, start_col) = if last_col >= grid_cols as i32 {
        (last_row + 1, 1)
    } else {
        (last_row, last_col + 1)
    };

    // Scan from start position to end of grid
    for row in start_row..=(grid_rows as i32) {
        let col_start = if row == start_row { start_col } else { 1 };
        for col in col_start..=(grid_cols as i32) {
            // Check if the box would fit within grid bounds
            let end_row = row + dim_height - 1;
            let end_col = col + dim_width - 1;

            if end_row > grid_rows as i32 || end_col > grid_cols as i32 {
                continue; // Box doesn't fit within grid bounds at this position
            }

            // Check if all cells needed by this box are free
            let mut all_free = true;
            for r in row..=end_row {
                for c in col..=end_col {
                    if occupied.contains(&(r, c)) {
                        all_free = false;
                        break;
                    }
                }
                if !all_free {
                    break;
                }
            }

            if all_free {
                return Some((row, col));
            }
        }
    }

    // If no free position found, return None
    None
}

/// Convert an ast::BoxBody into a BoxDef, processing all items
fn convert_ast_box_body(
    body: &ast::BoxBody,
    box_def_map: &HashMap<String, &ast::BoxDef>,
    source: &str,
    filename: &str,
    debug_dir: Option<&str>,
    box_name: &str,
) -> Result<BoxDef, String> {
    // First pass: extract properties, ports, and arrows
    let (grid, title, color, margin, border_style, ports, arrows) = extract_box_items(body);
    let mut boxes: Vec<Box> = Vec::new();

    // Second pass: process box instances with auto-positioning
    // Track occupied grid cells for auto-positioning
    let mut occupied: HashSet<(i32, i32)> = HashSet::new();
    // Track the last position for auto-positioning
    let mut last_pos = (1, 0); // Start before (1, 1)

    for item in &body.items {
        if let ast::BoxItem::BoxInst(box_inst) = item {
            match box_inst {
                ast::BoxInst::WithBody(with_body) => {
                    let box_def = process_inline_box(
                        with_body,
                        box_def_map,
                        source,
                        filename,
                        debug_dir,
                        box_name,
                        grid,
                        &mut occupied,
                        &mut last_pos,
                    )?;
                    boxes.push(box_def);
                }
                ast::BoxInst::Reference(reference) => {
                    let box_def = process_box_reference(
                        reference,
                        box_def_map,
                        source,
                        filename,
                        debug_dir,
                        grid,
                        &mut occupied,
                        &mut last_pos,
                    )?;
                    boxes.push(box_def);
                }
            }
        }
    }

    // Route arrows using A* pathfinding
    let routed_arrow_paths =
        route_arrows(&arrows, &ports, &boxes, grid, margin, debug_dir, box_name);

    Ok(BoxDef {
        grid,
        title,
        color,
        margin,
        border_style,
        boxes,
        ports,
        arrows,
        routed_arrow_paths,
    })
}

/// Route arrows using A* pathfinding
fn route_arrows(
    arrows: &[Arrow],
    ports: &[Port],
    boxes: &[Box],
    grid: (usize, usize),
    parent_margin: Option<f64>,
    debug_dir: Option<&str>,
    box_name: &str,
) -> Vec<Vec<(f64, f64)>> {
    use crate::routing::{ArrowRouter, BoundingBox};

    // Build port map (using f64 coordinates)
    let mut port_map: HashMap<String, (f64, f64)> = HashMap::new();
    for port in ports {
        port_map.insert(port.name.clone(), (port.coords.0, port.coords.1));
    }

    // Build bounding boxes for child boxes
    let mut bounding_boxes = Vec::new();
    for child_box in boxes {
        let (row, col) = child_box.pos;
        let (height, width) = child_box.dim;

        // Box at position (row, col) with dimensions (height, width)
        // Note: pos is 0-based indexing

        // Get margin from parent box
        // Margin is a scale factor (default 0.1 = 10% of cell size)
        // In fractional coordinates, 10% of a 1.0 cell = 0.1 units
        let margin_scale = parent_margin.unwrap_or(0.1);
        let margin = margin_scale * 0.1;

        let min_row = row as f64 + margin;
        let min_col = col as f64 + margin;
        let max_row = (row + height) as f64 - margin;
        let max_col = (col + width) as f64 - margin;

        // Store bounding box in fractional coordinates
        // The ArrowRouter will scale these by its grid_resolution
        bounding_boxes.push(BoundingBox {
            min_frac: (min_row, min_col),
            max_frac: (max_row, max_col),
        });
    }

    // Create router
    let mut router = ArrowRouter::new(
        grid.1 as f64, // grid width
        grid.0 as f64, // grid height
        bounding_boxes,
    );

    // Set debug directory if provided
    if let Some(dir) = debug_dir {
        router.set_debug_dir(dir, box_name);
    }

    // Get grid resolution from router
    let grid_resolution = router.grid_resolution();

    // Route each arrow
    let mut routed_paths = Vec::new();
    for arrow in arrows {
        if let (Some(&start), Some(&end)) = (port_map.get(&arrow.from), port_map.get(&arrow.to)) {
            if let Some(path) = router.route(start, end) {
                // Convert i32 points back to f64 for storage
                let f64_points: Vec<(f64, f64)> = path
                    .points
                    .iter()
                    .map(|(row, col)| {
                        (
                            *row as f64 / grid_resolution as f64,
                            *col as f64 / grid_resolution as f64,
                        )
                    })
                    .collect();
                routed_paths.push(f64_points);
            } else {
                // Fallback to straight line if routing fails
                routed_paths.push(vec![start, end]);
            }
        } else {
            // Port not found, push empty path
            routed_paths.push(Vec::new());
        }
    }

    routed_paths
}
