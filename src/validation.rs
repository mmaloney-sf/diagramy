// Validation for the AST

use crate::ast::{Document, Prop, BoxBody, BoxItem, Port};
use std::collections::HashSet;

// Valid colors from the color table in lib.rs
const VALID_COLORS: &[&str] = &[
    "transparent", "red", "blue", "green", "yellow", "orange", "purple", "pink",
    "cyan", "magenta", "lime", "teal", "indigo", "brown", "gray", "grey",
    "black", "white", "navy", "maroon", "olive",
];

// Valid diagram-level properties
const VALID_DIAGRAM_PROPS: &[&str] = &["width", "color", "title", "top", "version", "cheatPorts", "debug"];

// Valid box-level properties
const VALID_BOX_PROPS: &[&str] = &["grid", "text", "title", "color", "margin", "borderStyle", "bold", "debug"];

// Valid border styles
const VALID_BORDER_STYLES: &[&str] = &["solid", "none", "dotted", "dashed"];

/// Validate the entire document
pub fn validate(doc: &Document, source: &str, filename: &str) -> Result<(), String> {
    // Validate diagram properties
    validate_diagram_props(&doc.diagram.props, filename)?;

    // Validate all box definitions
    for box_def in &doc.box_defs {
        validate_box_body(&box_def.body, filename)?;
    }

    // Validate that the top: property references an existing box
    validate_top_property(doc, source, filename)?;

    // Validate that all box references exist
    validate_box_references(doc, filename)?;

    Ok(())
}

/// Validate diagram-level properties
fn validate_diagram_props(props: &[Prop], filename: &str) -> Result<(), String> {
    for prop in props {
        let span = prop.span();
        let start = span.start();
        let key = match prop {
            Prop::PropIdent(p) => &p.key,
            Prop::PropString(p) => &p.key,
            Prop::PropNumber(p) => &p.key,
            Prop::PropFrac(p) => &p.key,
            Prop::PropCoords(p) => &p.key,
            Prop::PropDim(p) => &p.key,
        };

        // Check if property is known
        if !VALID_DIAGRAM_PROPS.contains(&key.as_str()) {
            return Err(format!(
                "{}:{}:{}: Unknown diagram property: '{}'. Valid properties are: {}",
                filename,
                start.line(),
                start.col(),
                key,
                VALID_DIAGRAM_PROPS.join(", ")
            ));
        }

        // Validate property types
        match key.as_str() {
            "width" => {
                if !matches!(prop, Prop::PropNumber(_)) {
                    return Err(format!("{}:{}:{}: Property 'width' must be a number", filename, start.line(), start.col()));
                }
            }
            "color" => {
                if let Prop::PropIdent(p) = prop {
                    validate_color(&p.value, filename, span)?;
                } else {
                    return Err(format!("{}:{}:{}: Property 'color' must be an identifier", filename, start.line(), start.col()));
                }
            }
            "title" => {
                if !matches!(prop, Prop::PropString(_)) {
                    return Err(format!("{}:{}:{}: Property 'title' must be a string", filename, start.line(), start.col()));
                }
            }
            "top" => {
                if !matches!(prop, Prop::PropIdent(_)) {
                    return Err(format!("{}:{}:{}: Property 'top' must be an identifier", filename, start.line(), start.col()));
                }
            }
            "version" => {
                if !matches!(prop, Prop::PropString(_)) {
                    return Err(format!("{}:{}:{}: Property 'version' must be a string", filename, start.line(), start.col()));
                }
            }
            "cheatPorts" => {
                if !matches!(prop, Prop::PropIdent(p) if p.value == "true" || p.value == "false") {
                    return Err(format!("{}:{}:{}: Property 'cheatPorts' must be 'true' or 'false'", filename, start.line(), start.col()));
                }
            }
            "debug" => {
                if !matches!(prop, Prop::PropIdent(p) if p.value == "true" || p.value == "false") {
                    return Err(format!("{}:{}:{}: Property 'debug' must be 'true' or 'false'", filename, start.line(), start.col()));
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Validate box body (recursively validates nested boxes)
fn validate_box_body(body: &BoxBody, filename: &str) -> Result<(), String> {
    // First validate properties
    for item in &body.items {
        match item {
            BoxItem::Prop(prop) => validate_box_prop(prop, filename)?,
            BoxItem::BoxInst(box_inst) => {
                // Recursively validate nested boxes
                match box_inst {
                    crate::ast::BoxInst::WithBody(with_body) => {
                        validate_box_body(&with_body.body, filename)?;
                    }
                    crate::ast::BoxInst::Reference(_) => {
                        // References are validated during elaboration
                    }
                }
            }
            BoxItem::Port(port) => {
                // Validate port body if present
                if let Some(ref port_body) = port.body {
                    // Disallow boxes inside port bodies
                    validate_no_boxes_in_body(port_body, "port", &port.name, filename)?;
                    validate_box_body(port_body, filename)?;
                }
                // Validate port coordinates are in bounds
                validate_port_bounds(port, body, filename)?;
                // Validate port is not inside child boxes
                validate_port_not_in_child_boxes(port, body, filename)?;
                // Validate port is not too close to corners
                validate_port_not_near_corners(port, body, filename)?;
            }
            BoxItem::Arrow(arrow) => {
                // Validate arrow body if present
                if let Some(ref arrow_body) = arrow.body {
                    // Disallow boxes inside arrow bodies
                    let arrow_name = format!("{:?} to {:?}", arrow.from, arrow.to);
                    validate_no_boxes_in_body(arrow_body, "arrow", &arrow_name, filename)?;
                    validate_box_body(arrow_body, filename)?;
                }
            }
            BoxItem::Label(_label) => {
                // Labels are converted to boxes during elaboration, no special validation needed
            }
        }
    }

    // Validate box positions if this box has a grid
    validate_box_positions(body, filename)?;

    // Validate that no two boxes have the same name
    validate_unique_box_names(body, filename)?;

    // Validate that boxes don't have both text property and child boxes
    validate_text_and_children_conflict(body, filename)?;

    // Validate that boxes with child boxes have a grid property
    validate_grid_required_for_children(body, filename)?;

    // Validate no name conflicts among boxes, ports, and arrows
    validate_no_name_conflicts(body, filename)?;

    Ok(())
}

/// Validate a box-level property
fn validate_box_prop(prop: &Prop, filename: &str) -> Result<(), String> {
    let span = prop.span();
    let start = span.start();
    let key = match prop {
        Prop::PropIdent(p) => &p.key,
        Prop::PropString(p) => &p.key,
        Prop::PropNumber(p) => &p.key,
        Prop::PropFrac(p) => &p.key,
        Prop::PropCoords(p) => &p.key,
        Prop::PropDim(p) => &p.key,
    };

    // Forbid 'text' property in boxes - it's only for labels
    if key == "text" {
        return Err(format!(
            "{}:{}:{}: Property 'text' is not allowed in boxes. Use a label element instead.",
            filename,
            start.line(),
            start.col()
        ));
    }

    // Check if property is known
    if !VALID_BOX_PROPS.contains(&key.as_str()) {
        return Err(format!(
            "{}:{}:{}: Unknown box property: '{}'. Valid properties are: {}",
            filename,
            start.line(),
            start.col(),
            key,
            VALID_BOX_PROPS.join(", ")
        ));
    }

    // Validate property types
    match key.as_str() {
        "grid" => {
            if !matches!(prop, Prop::PropDim(_)) {
                return Err(format!("{}:{}:{}: Property 'grid' must be dimensions (heightxwidth)", filename, start.line(), start.col()));
            }
        }
        "text" => {
            if !matches!(prop, Prop::PropString(_)) {
                return Err(format!("{}:{}:{}: Property 'text' must be a string", filename, start.line(), start.col()));
            }
        }
        "color" => {
            if let Prop::PropIdent(p) = prop {
                validate_color(&p.value, filename, span)?;
            } else {
                return Err(format!("{}:{}:{}: Property 'color' must be an identifier", filename, start.line(), start.col()));
            }
        }
        "margin" => {
            let margin_value = match prop {
                Prop::PropNumber(p) => p.value as f64,
                Prop::PropFrac(p) => p.value,
                _ => {
                    return Err(format!("{}:{}:{}: Property 'margin' must be a number", filename, start.line(), start.col()));
                }
            };
            if margin_value < 0.0 || margin_value > 0.5 {
                return Err(format!(
                    "{}:{}:{}: Property 'margin' must be between 0.0 and 0.5, got {}",
                    filename,
                    start.line(),
                    start.col(),
                    margin_value
                ));
            }
        }
        "borderStyle" => {
            if let Prop::PropIdent(p) = prop {
                if !VALID_BORDER_STYLES.contains(&p.value.as_str()) {
                    return Err(format!(
                        "{}:{}:{}: Unknown borderStyle: '{}'. Valid styles are: {}",
                        filename,
                        start.line(),
                        start.col(),
                        p.value,
                        VALID_BORDER_STYLES.join(", ")
                    ));
                }
            } else {
                return Err(format!("{}:{}:{}: Property 'borderStyle' must be an identifier", filename, start.line(), start.col()));
            }
        }
        "bold" => {
            if !matches!(prop, Prop::PropIdent(p) if p.value == "true" || p.value == "false") {
                return Err(format!("{}:{}:{}: Property 'bold' must be 'true' or 'false'", filename, start.line(), start.col()));
            }
        }
        _ => {}
    }

    Ok(())
}

/// Validate that a color is in the valid color table
fn validate_color(color: &str, filename: &str, span: crate::ast::Span) -> Result<(), String> {
    if !VALID_COLORS.contains(&color) {
        let start = span.start();
        return Err(format!(
            "{}:{}:{}: Unknown color: '{}'. Valid colors are: {}",
            filename,
            start.line(),
            start.col(),
            color,
            VALID_COLORS.join(", ")
        ));
    }
    Ok(())
}

/// Extract grid size from box properties
fn get_grid_size(body: &BoxBody) -> Option<crate::ast::Dim> {
    for item in &body.items {
        if let BoxItem::Prop(Prop::PropDim(p)) = item {
            if p.key == "grid" {
                return Some(p.value.clone());
            }
        }
    }
    None
}

/// Validate that all child box positions are within grid bounds and don't overlap
fn validate_box_positions(body: &BoxBody, filename: &str) -> Result<(), String> {
    // Get the grid size if it exists
    let grid_size = match get_grid_size(body) {
        Some(grid) => grid,
        None => return Ok(()), // No grid, no position validation needed
    };

    // Track which cells are occupied
    let mut occupied_cells: HashSet<(i32, i32)> = HashSet::new();
    // Track the last position for auto-positioning
    let mut last_pos = (1, 0); // Start before (1, 1)

    for item in &body.items {
        if let BoxItem::BoxInst(box_inst) = item {
            let span = box_inst.span();
            let start = span.start();
            let (coords_opt, dim_opt) = match box_inst {
                crate::ast::BoxInst::WithBody(with_body) => (&with_body.coords, &with_body.dim),
                crate::ast::BoxInst::Reference(reference) => (&reference.coords, &reference.dim),
            };

            // If dim is not specified, default to 1x1 for validation purposes
            // (elaboration will set it to the child's grid, but we don't have that here)
            let dim = if let Some(d) = dim_opt {
                (d.height, d.width)
            } else {
                (1, 1)
            };

            // Determine actual position (explicit or auto-positioned)
            let (row, col) = if let Some(c) = coords_opt {
                (c.row, c.col)
            } else {
                // Auto-positioned box - find next free position
                match find_next_free_position(&occupied_cells, (grid_size.height, grid_size.width), dim, last_pos) {
                    Some(pos) => {
                        last_pos = pos;
                        pos
                    }
                    None => {
                        return Err(format!(
                            "{}:{}:{}: Cannot auto-position box with dim {}x{}. No free space available in {}x{} grid",
                            filename, start.line(), start.col(), dim.0, dim.1, grid_size.height, grid_size.width
                        ));
                    }
                }
            };

            // Check if position is within grid bounds (1-based indexing)
            if row < 1 || row > grid_size.height {
                return Err(format!(
                    "{}:{}:{}: Box position ({}, {}) is out of bounds. Grid size is {}x{}, so row must be in range [1, {}]",
                    filename, start.line(), start.col(), row, col, grid_size.height, grid_size.width, grid_size.height
                ));
            }

            if col < 1 || col > grid_size.width {
                return Err(format!(
                    "{}:{}:{}: Box position ({}, {}) is out of bounds. Grid size is {}x{}, so col must be in range [1, {}]",
                    filename, start.line(), start.col(), row, col, grid_size.height, grid_size.width, grid_size.width
                ));
            }

            // Check if box with dim fits within grid bounds (1-based indexing)
            // For 1-based indexing, a box at (1, 1) with dim 1x2 occupies cells (1, 1) and (1, 2)
            let end_row = row + dim.0 - 1;
            let end_col = col + dim.1 - 1;

            if end_row > grid_size.height {
                return Err(format!(
                    "{}:{}:{}: Box at ({}, {}) with dim {}x{} extends beyond grid bounds. End row {} exceeds grid height {}",
                    filename, start.line(), start.col(), row, col, dim.0, dim.1, end_row, grid_size.height
                ));
            }

            if end_col > grid_size.width {
                return Err(format!(
                    "{}:{}:{}: Box at ({}, {}) with dim {}x{} extends beyond grid bounds. End col {} exceeds grid width {}",
                    filename, start.line(), start.col(), row, col, dim.0, dim.1, end_col, grid_size.width
                ));
            }

            // Check for overlapping cells
            for r in row..=end_row {
                for c in col..=end_col {
                    let cell = (r, c);
                    if occupied_cells.contains(&cell) {
                        return Err(format!(
                            "{}:{}:{}: Box at ({}, {}) with dim {}x{} overlaps with another box at cell ({}, {})",
                            filename, start.line(), start.col(), row, col, dim.0, dim.1, r, c
                        ));
                    }
                    occupied_cells.insert(cell);
                }
            }
        }
    }

    Ok(())
}

/// Validate that no two boxes have the same name within a parent
fn validate_unique_box_names(body: &BoxBody, filename: &str) -> Result<(), String> {
    let mut names: HashSet<String> = HashSet::new();

    for item in &body.items {
        if let BoxItem::BoxInst(box_inst) = item {
            let span = box_inst.span();
            let start = span.start();
            let id = match box_inst {
                crate::ast::BoxInst::WithBody(with_body) => with_body.id.as_ref(),
                crate::ast::BoxInst::Reference(reference) => reference.id.as_ref(),
            };

            // Only check named boxes
            if let Some(name) = id {
                if names.contains(name) {
                    return Err(format!(
                        "{}:{}:{}: Duplicate box name '{}'. Each box must have a unique name within its parent",
                        filename, start.line(), start.col(), name
                    ));
                }
                names.insert(name.clone());
            }
        }
    }

    Ok(())
}

/// Validate that boxes don't have both text property and children (boxes or ports)
fn validate_text_and_children_conflict(body: &BoxBody, filename: &str) -> Result<(), String> {
    // Check if this box has a text property
    let text_prop = body.items.iter().find_map(|item| {
        if let BoxItem::Prop(Prop::PropString(p)) = item {
            if p.key == "text" {
                return Some(p.span);
            }
        }
        None
    });

    // Check if this box has child boxes
    let child_box = body.items.iter().find_map(|item| {
        if let BoxItem::BoxInst(box_inst) = item {
            return Some(box_inst.span());
        }
        None
    });

    // Check if this box has ports
    let port = body.items.iter().find_map(|item| {
        if let BoxItem::Port(port) = item {
            return Some(port.span);
        }
        None
    });

    // If text and child boxes exist, return an error
    if let (Some(text_span), Some(_child_span)) = (text_prop, child_box) {
        let text_start = text_span.start();
        return Err(format!(
            "{}:{}:{}: Box cannot have both 'text:' property and child boxes. Consider using a box with borderStyle: none to position a label.",
            filename,
            text_start.line(),
            text_start.col()
        ));
    }

    // If text and ports exist, return an error
    if let (Some(text_span), Some(_port_span)) = (text_prop, port) {
        let text_start = text_span.start();
        return Err(format!(
            "{}:{}:{}: Box cannot have both 'text:' property and ports",
            filename,
            text_start.line(),
            text_start.col()
        ));
    }

    Ok(())
}

/// Validate that boxes with grandchild boxes have a grid property
fn validate_grid_required_for_children(body: &BoxBody, filename: &str) -> Result<(), String> {
    // Check if this box has grandchild boxes (children that have their own child boxes)
    // Note: grandchild labels don't count, only grandchild boxes
    let grandchild_span = body.items.iter().find_map(|item| {
        if let BoxItem::BoxInst(box_inst) = item {
            // Check if this child box has its own child boxes
            let child_body = match box_inst {
                crate::ast::BoxInst::WithBody(with_body) => Some(&with_body.body),
                crate::ast::BoxInst::Reference(_) => None, // References don't have inline bodies to check
            };

            if let Some(child_body) = child_body {
                // Check if the child has any box instances (labels don't count)
                let has_child_boxes = child_body.items.iter().any(|child_item| {
                    matches!(child_item, BoxItem::BoxInst(_))
                });

                if has_child_boxes {
                    return Some(box_inst.span());
                }
            }
        }
        None
    });

    // If there are grandchild boxes, check for grid property
    if let Some(grandchild_span) = grandchild_span {
        let has_grid = body.items.iter().any(|item| {
            if let BoxItem::Prop(Prop::PropDim(p)) = item {
                p.key == "grid"
            } else {
                false
            }
        });

        if !has_grid {
            let grandchild_start = grandchild_span.start();
            return Err(format!(
                "{}:{}:{}: Box with grandchild boxes must have a 'grid:' property",
                filename,
                grandchild_start.line(),
                grandchild_start.col()
            ));
        }
    }

    Ok(())
}

/// Validate that there are no name conflicts among boxes, ports, and arrows within a box
fn validate_no_name_conflicts(body: &BoxBody, filename: &str) -> Result<(), String> {
    use std::collections::HashMap;

    // Track all names and their spans
    let mut names: HashMap<String, (crate::ast::Span, &str)> = HashMap::new();

    for item in &body.items {
        match item {
            BoxItem::BoxInst(box_inst) => {
                let span = box_inst.span();
                let id = match box_inst {
                    crate::ast::BoxInst::WithBody(with_body) => with_body.id.as_ref(),
                    crate::ast::BoxInst::Reference(reference) => reference.id.as_ref(),
                };

                if let Some(name) = id {
                    if let Some((prev_span, prev_type)) = names.get(name) {
                        let start = span.start();
                        let prev_start = prev_span.start();
                        return Err(format!(
                            "{}:{}:{}: Name conflict: '{}' is already used by a {} at line {}",
                            filename,
                            start.line(),
                            start.col(),
                            name,
                            prev_type,
                            prev_start.line()
                        ));
                    }
                    names.insert(name.clone(), (span, "box"));
                }
            }
            BoxItem::Port(port) => {
                let name = &port.name;
                let span = port.span;

                if let Some((prev_span, prev_type)) = names.get(name) {
                    let start = span.start();
                    let prev_start = prev_span.start();
                    return Err(format!(
                        "{}:{}:{}: Name conflict: '{}' is already used by a {} at line {}",
                        filename,
                        start.line(),
                        start.col(),
                        name,
                        prev_type,
                        prev_start.line()
                    ));
                }
                names.insert(name.clone(), (span, "port"));
            }
            BoxItem::Arrow(_arrow) => {
                // Arrows can have names if they use simple identifiers (not paths)
                // For now, we'll skip arrow name validation since arrows reference ports/boxes
                // and don't have their own names in the current grammar
            }
            BoxItem::Prop(_) => {
                // Properties don't have names that conflict
            }
            BoxItem::Label(_label) => {
                // Labels are converted to boxes during elaboration, no name conflicts
            }
        }
    }

    Ok(())
}

/// Validate that the top: property references an existing box definition
fn validate_top_property(doc: &Document, source: &str, filename: &str) -> Result<(), String> {
    // Find the top: property in diagram props
    for prop in &doc.diagram.props {
        if let Prop::PropIdent(p) = prop {
            if p.key == "top" {
                // Check if a box with this name exists
                let box_exists = doc.box_defs.iter().any(|bd| bd.name == p.value);
                if !box_exists {
                    // Get the span for the value to report the error at the right location
                    let value_span = crate::ast::Span::from_offsets(source, p.value_location.0, p.value_location.1);
                    let start = value_span.start();
                    return Err(format!(
                        "{}:{}:{}: No such box: {}",
                        filename,
                        start.line(),
                        start.col(),
                        p.value
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Validate that all box references point to existing box definitions
fn validate_box_references(doc: &Document, filename: &str) -> Result<(), String> {
    // Build a set of all box definition names for quick lookup
    let box_names: HashSet<String> = doc.box_defs.iter().map(|bd| bd.name.clone()).collect();

    // Check all box definitions
    for box_def in &doc.box_defs {
        validate_box_body_references(&box_def.body, &box_names, filename)?;
    }

    Ok(())
}

/// Recursively validate box references in a box body
fn validate_box_body_references(
    body: &BoxBody,
    box_names: &HashSet<String>,
    filename: &str,
) -> Result<(), String> {
    for item in &body.items {
        if let BoxItem::BoxInst(box_inst) = item {
            match box_inst {
                crate::ast::BoxInst::WithBody(with_body) => {
                    // Recursively validate nested box bodies
                    validate_box_body_references(&with_body.body, box_names, filename)?;
                }
                crate::ast::BoxInst::Reference(reference) => {
                    // Check if the referenced box exists
                    if !box_names.contains(&reference.def_name) {
                        let start = reference.span.start();
                        return Err(format!(
                            "{}:{}:{}: No such box: {}",
                            filename,
                            start.line(),
                            start.col(),
                            reference.def_name
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Validate that port coordinates are within bounds
fn validate_port_bounds(port: &Port, body: &BoxBody, filename: &str) -> Result<(), String> {
    // Validate mutual exclusivity of "at" and "on" clauses
    if port.coords.is_some() && port.on.is_some() {
        let start = port.span.start();
        return Err(format!(
            "{}:{}:{}: Port '{}' cannot have both 'at' and 'on' clauses",
            filename,
            start.line(),
            start.col(),
            port.name
        ));
    }

    // Note: "on" clause validation is now handled by the Side enum type
    // No need to validate the value here since the parser ensures it's valid

    // If "at" clause is present, validate coordinates are within bounds
    if let Some(ref coords) = port.coords {
        // Extract the grid dimensions from the box body
        let mut grid = (1, 1); // default grid
        for item in &body.items {
            if let BoxItem::Prop(Prop::PropDim(p)) = item {
                if p.key == "grid" {
                    grid = (p.value.height, p.value.width);
                    break;
                }
            }
        }

        let (height, width) = grid;
        let port_span = coords.span;
        let start = port_span.start();

        // Validate row (y-coordinate) is in bounds [1, HEIGHT]
        if coords.row < 1 {
            return Err(format!(
                "{}:{}:{}: Port '{}' row coordinate {} is out of bounds (must be >= 1)",
                filename,
                start.line(),
                start.col(),
                port.name,
                coords.row
            ));
        }
        if coords.row > height {
            return Err(format!(
                "{}:{}:{}: Port '{}' row coordinate {} is out of bounds (must be <= {} for grid {}x{})",
                filename,
                start.line(),
                start.col(),
                port.name,
                coords.row,
                height,
                height,
                width
            ));
        }

        // Validate col (x-coordinate) is in bounds [1, WIDTH]
        if coords.col < 1 {
            return Err(format!(
                "{}:{}:{}: Port '{}' col coordinate {} is out of bounds (must be >= 1)",
                filename,
                start.line(),
                start.col(),
                port.name,
                coords.col
            ));
        }
        if coords.col > width {
            return Err(format!(
                "{}:{}:{}: Port '{}' col coordinate {} is out of bounds (must be <= {} for grid {}x{})",
                filename,
                start.line(),
                start.col(),
                port.name,
                coords.col,
                width,
                height,
                width
            ));
        }
    }

    Ok(())
}

/// Find the next free grid position that can fit a box with the given dimensions
/// Starts scanning from the position FOLLOWING last_pos
/// Returns Some((row, col)) in 1-based indexing, or None if no position found
fn find_next_free_position(occupied: &HashSet<(i32, i32)>, grid: (i32, i32), dim: (i32, i32), last_pos: (i32, i32)) -> Option<(i32, i32)> {
    let (grid_rows, grid_cols) = grid;
    let (dim_height, dim_width) = dim;
    let (last_row, last_col) = last_pos;

    // Calculate the starting position (next position after last_pos)
    let (start_row, start_col) = if last_col >= grid_cols {
        (last_row + 1, 1)
    } else {
        (last_row, last_col + 1)
    };

    // Scan from start position to end of grid
    for row in start_row..=grid_rows {
        let col_start = if row == start_row { start_col } else { 1 };
        for col in col_start..=grid_cols {
            // Check if the box would fit within grid bounds
            let end_row = row + dim_height - 1;
            let end_col = col + dim_width - 1;

            if end_row > grid_rows || end_col > grid_cols {
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

/// Validate that a port is not inside any child boxes (excluding margins)
fn validate_port_not_in_child_boxes(port: &Port, body: &BoxBody, filename: &str) -> Result<(), String> {
    use crate::ast::{BoxItem, BoxInst, Prop};

    // Get grid dimensions
    let mut grid = (1, 1);
    for item in &body.items {
        if let BoxItem::Prop(Prop::PropDim(p)) = item {
            if p.key == "grid" {
                grid = (p.value.height, p.value.width);
                break;
            }
        }
    }

    let (grid_height, grid_width) = grid;

    // Track occupied cells for auto-positioning
    let mut occupied: HashSet<(i32, i32)> = HashSet::new();
    // Track the last position for auto-positioning
    let mut last_pos = (1, 0); // Start before (1, 1)

    // Check each child box
    for item in &body.items {
        if let BoxItem::BoxInst(box_inst) = item {
            // Get position and dimensions of the child box
            let (coords_opt, dimensions_opt) = match box_inst {
                BoxInst::WithBody(with_body) => (with_body.coords.as_ref(), &with_body.dim),
                BoxInst::Reference(reference) => (reference.coords.as_ref(), &reference.dim),
            };

            // Default to 1x1 if not specified (elaboration will use child's grid)
            let dimensions = if let Some(d) = dimensions_opt {
                (d.height, d.width)
            } else {
                (1, 1)
            };

            // Determine actual position (explicit or auto-positioned)
            let (child_row, child_col) = if let Some(coords) = coords_opt {
                (coords.row, coords.col)
            } else {
                // Auto-positioned box - find next free position
                match find_next_free_position(&occupied, grid, dimensions, last_pos) {
                    Some(pos) => pos,
                    None => {
                        // No free space - return error
                        let span = box_inst.span();
                        let start = span.start();
                        return Err(format!(
                            "{}:{}:{}: Cannot auto-position box with dim {}x{}. No free space available in {}x{} grid",
                            filename, start.line(), start.col(), dimensions.0, dimensions.1, grid.0, grid.1
                        ));
                    }
                }
            };

            // Update last position
            last_pos = (child_row, child_col);

            // Mark occupied cells
            for r in child_row..(child_row + dimensions.0) {
                for c in child_col..(child_col + dimensions.1) {
                    occupied.insert((r, c));
                }
            }

            // Check if port overlaps with this child box
            // Only check ports with explicit "at" coordinates (ports with "on" are on borders, can't overlap)
            if let Some(ref coords) = port.coords {
                // Port coords are 1-based integers
                // Child box occupies cells from child_row to (child_row + dimensions.0 - 1)
                //                         and child_col to (child_col + dimensions.1 - 1)
                let child_row_end = child_row + dimensions.0 - 1;
                let child_col_end = child_col + dimensions.1 - 1;

                // Check if port position falls within the child box area (inclusive)
                if coords.row >= child_row && coords.row <= child_row_end &&
                   coords.col >= child_col && coords.col <= child_col_end {
                    let port_span = coords.span;
                    let start = port_span.start();
                    return Err(format!(
                        "{}:{}:{}: Port '{}' at ({}, {}) overlaps with child box at ({}, {}) with dim {}x{}",
                        filename,
                        start.line(),
                        start.col(),
                        port.name,
                        coords.row,
                        coords.col,
                        child_row,
                        child_col,
                        dimensions.0,
                        dimensions.1
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Validate that a port is not too close to corners
fn validate_port_not_near_corners(port: &Port, body: &BoxBody, filename: &str) -> Result<(), String> {
    use crate::ast::{BoxItem, Prop};

    // Only validate if port has explicit "at" coordinates
    // Ports with "on" clause are auto-positioned and won't be at corners
    let coords = match &port.coords {
        Some(c) => c,
        None => return Ok(()), // Skip validation for "on" ports
    };

    // Get grid dimensions
    let mut grid = (1, 1);
    for item in &body.items {
        if let BoxItem::Prop(Prop::PropDim(p)) = item {
            if p.key == "grid" {
                grid = (p.value.height, p.value.width);
                break;
            }
        }
    }

    let (height, width) = grid;

    // Define padding as a small distance from corners
    // Using a fixed padding value (could be made configurable)
    let padding = 0.1; // 10% of a grid cell

    let port_row = coords.row as f64;
    let port_col = coords.col as f64;

    // Check distance from each corner
    let corners = [
        (0.0, 0.0, "upper-left"),
        (0.0, width as f64, "upper-right"),
        (height as f64, 0.0, "lower-left"),
        (height as f64, width as f64, "lower-right"),
    ];

    for (corner_row, corner_col, corner_name) in &corners {
        let distance = ((port_row - corner_row).powi(2) + (port_col - corner_col).powi(2)).sqrt();

        if distance < padding {
            let port_span = coords.span;
            let start = port_span.start();
            return Err(format!(
                "{}:{}:{}: Port '{}' at ({}, {}) is too close to the {} corner ({}, {})",
                filename,
                start.line(),
                start.col(),
                port.name,
                coords.row,
                coords.col,
                corner_name,
                corner_row,
                corner_col
            ));
        }
    }

    Ok(())
}

/// Validate that a body (port or arrow) does not contain box instances
fn validate_no_boxes_in_body(
    body: &BoxBody,
    container_type: &str,
    container_name: &str,
    filename: &str,
) -> Result<(), String> {
    use crate::ast::BoxItem;

    for item in &body.items {
        if let BoxItem::BoxInst(box_inst) = item {
            let start = box_inst.span().start();
            return Err(format!(
                "{}:{}:{}: Box instances are not allowed inside {} '{}' body",
                filename,
                start.line(),
                start.col(),
                container_type,
                container_name
            ));
        }
    }

    Ok(())
}


