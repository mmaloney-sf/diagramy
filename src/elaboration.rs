use std::sync::Arc;
use std::collections::HashMap;
use std::collections::HashSet;
use crate::ast;

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
}

#[derive(Debug)]
pub struct Box {
    pub def: Arc<BoxDef>,
    pub pos: (usize, usize),
    pub dim: (usize, usize), // (height, width) - number of grid cells to span
}

/// Convert an ast::Document into a diagram::Diagram
pub fn from_ast(doc: &ast::Document, source: &str, filename: &str) -> Result<ElaboratedDiagram, String> {
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
            ast::Prop::PropString { key, value, .. } if key == "text" => {
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
    // 2. Otherwise, if a box named "top" exists, use it
    // 3. Otherwise, use the first box definition
    // 4. If no box definitions exist, error out
    let top_ast_def = if let Some(ref name) = top_name {
        // top: property was specified, look it up
        box_def_map.get(name).copied()
            .ok_or_else(|| format!("{}:0:0: No such box: {}", filename, name))?
    } else {
        // No top: property, try "top" box or first box
        box_def_map.get("top").copied()
            .or_else(|| doc.box_defs.first())
            .ok_or_else(|| format!("{}:0:0: Document must have at least one box definition", filename))?
    };

    // Convert the top box definition
    let top_box_def = convert_ast_box_body(&top_ast_def.body, &box_def_map, source, filename)?;

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

/// Convert byte offset to line and column numbers
/// This function is deprecated - use Span::from_offsets instead
#[allow(dead_code)]
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;

    for (i, ch) in source.chars().enumerate() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

/// Find the next free grid position that can fit a box with the given dimensions
/// Returns Some((row, col)) in 1-based indexing, or None if no position found
fn find_next_free_position(occupied: &HashSet<(i32, i32)>, grid: (usize, usize), dim: (i32, i32)) -> Option<(i32, i32)> {
    let (grid_rows, grid_cols) = grid;
    let (dim_height, dim_width) = dim;

    // Scan from (1,1) to (1,n), then (2,1) to (2,n), etc.
    for row in 1..=(grid_rows as i32) {
        for col in 1..=(grid_cols as i32) {
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
fn convert_ast_box_body(body: &ast::BoxBody, box_def_map: &HashMap<String, &ast::BoxDef>, source: &str, filename: &str) -> Result<BoxDef, String> {
    let mut grid = (1, 1); // default grid
    let mut title: Option<String> = None;
    let mut color: Option<String> = None;
    let mut margin: Option<f64> = None;
    let mut border_style: Option<String> = None;
    let mut boxes: Vec<Box> = Vec::new();

    // First pass: extract properties
    for item in &body.items {
        if let ast::BoxItem::Prop(prop) = item {
            match prop {
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
            }
        }
    }

    // Second pass: process box instances with auto-positioning
    // Track occupied grid cells for auto-positioning
    let mut occupied: HashSet<(i32, i32)> = HashSet::new();

    for item in &body.items {
        if let ast::BoxItem::BoxInst(box_inst) = item {
            match box_inst {
                ast::BoxInst::WithBody { id: _, coords, dim, body, span } => {
                    // Determine position (auto-position if coords is None)
                    let (row, col) = if let Some(c) = coords {
                        (c.row, c.col)
                    } else {
                        match find_next_free_position(&occupied, grid, (dim.height, dim.width)) {
                            Some(pos) => pos,
                            None => {
                                let start = span.start();
                                return Err(format!(
                                    "{}:{}:{}: Cannot auto-position box with dim {}x{}. No free space available in {}x{} grid",
                                    filename, start.line(), start.col(), dim.height, dim.width, grid.0, grid.1
                                ));
                            }
                        }
                    };

                    // Check for overlaps and mark occupied cells (including cells occupied by dim)
                    for r in row..(row + dim.height) {
                        for c in col..(col + dim.width) {
                            if occupied.contains(&(r, c)) {
                                let start = span.start();
                                return Err(format!(
                                    "{}:{}:{}: Box at ({}, {}) with dim {}x{} overlaps with another box at cell ({}, {})",
                                    filename, start.line(), start.col(), row, col, dim.height, dim.width, r, c
                                ));
                            }
                            occupied.insert((r, c));
                        }
                    }

                    // Recursively convert the nested box body
                    let nested_def = convert_ast_box_body(body, box_def_map, source, filename)?;
                    boxes.push(Box {
                        def: Arc::new(nested_def),
                        // Convert from 1-based to 0-based indexing
                        pos: ((row - 1) as usize, (col - 1) as usize),
                        dim: (dim.height as usize, dim.width as usize),
                    });
                }
                ast::BoxInst::Reference { id: _, coords, dim, def_name, location: _, span } => {
                    // Determine position (auto-position if coords is None)
                    let (row, col) = if let Some(c) = coords {
                        (c.row, c.col)
                    } else {
                        match find_next_free_position(&occupied, grid, (dim.height, dim.width)) {
                            Some(pos) => pos,
                            None => {
                                let start = span.start();
                                return Err(format!(
                                    "{}:{}:{}: Cannot auto-position box '{}' with dim {}x{}. No free space available in {}x{} grid",
                                    filename, start.line(), start.col(), def_name, dim.height, dim.width, grid.0, grid.1
                                ));
                            }
                        }
                    };

                    // Check for overlaps and mark occupied cells (including cells occupied by dim)
                    for r in row..(row + dim.height) {
                        for c in col..(col + dim.width) {
                            if occupied.contains(&(r, c)) {
                                let start = span.start();
                                return Err(format!(
                                    "{}:{}:{}: Box at ({}, {}) with dim {}x{} overlaps with another box at cell ({}, {})",
                                    filename, start.line(), start.col(), row, col, dim.height, dim.width, r, c
                                ));
                            }
                            occupied.insert((r, c));
                        }
                    }

                    // Look up the referenced box definition
                    if let Some(referenced_def) = box_def_map.get(def_name) {
                        let nested_def = convert_ast_box_body(&referenced_def.body, box_def_map, source, filename)?;
                        boxes.push(Box {
                            def: Arc::new(nested_def),
                            // Convert from 1-based to 0-based indexing
                            pos: ((row - 1) as usize, (col - 1) as usize),
                            dim: (dim.height as usize, dim.width as usize),
                        });
                    } else {
                        // Error: referenced box definition not found
                        // Use span information for better error reporting
                        let start = span.start();
                        return Err(format!("{}:{}:{}: No such box: {}", filename, start.line(), start.col(), def_name));
                    }
                }
            }
        }
    }

    Ok(BoxDef {
        grid,
        title,
        color,
        margin,
        border_style,
        boxes,
    })
}
