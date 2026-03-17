// Validation for the AST

use crate::ast::{Document, Prop, BoxBody, BoxItem};
use std::collections::HashSet;

// Valid colors from the color table in lib.rs
const VALID_COLORS: &[&str] = &[
    "transparent", "red", "blue", "green", "yellow", "orange", "purple", "pink",
    "cyan", "magenta", "lime", "teal", "indigo", "brown", "gray", "grey",
    "black", "white", "navy", "maroon", "olive",
];

// Valid diagram-level properties
const VALID_DIAGRAM_PROPS: &[&str] = &["width", "color", "text", "top", "version"];

// Valid box-level properties
const VALID_BOX_PROPS: &[&str] = &["grid", "text", "color", "margin", "borderStyle"];

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
            Prop::PropIdent { key, .. } => key,
            Prop::PropString { key, .. } => key,
            Prop::PropNumber { key, .. } => key,
            Prop::PropFrac { key, .. } => key,
            Prop::PropCoords { key, .. } => key,
            Prop::PropDim { key, .. } => key,
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
                if !matches!(prop, Prop::PropNumber { .. }) {
                    return Err(format!("{}:{}:{}: Property 'width' must be a number", filename, start.line(), start.col()));
                }
            }
            "color" => {
                if let Prop::PropIdent { value, .. } = prop {
                    validate_color(value, filename, span)?;
                } else {
                    return Err(format!("{}:{}:{}: Property 'color' must be an identifier", filename, start.line(), start.col()));
                }
            }
            "text" => {
                if !matches!(prop, Prop::PropString { .. }) {
                    return Err(format!("{}:{}:{}: Property 'text' must be a string", filename, start.line(), start.col()));
                }
            }
            "top" => {
                if !matches!(prop, Prop::PropIdent { .. }) {
                    return Err(format!("{}:{}:{}: Property 'top' must be an identifier", filename, start.line(), start.col()));
                }
            }
            "version" => {
                if !matches!(prop, Prop::PropString { .. }) {
                    return Err(format!("{}:{}:{}: Property 'version' must be a string", filename, start.line(), start.col()));
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
                    crate::ast::BoxInst::WithBody { body, .. } => {
                        validate_box_body(body, filename)?;
                    }
                    crate::ast::BoxInst::Reference { .. } => {
                        // References are validated during elaboration
                    }
                }
            }
            BoxItem::Port(port) => {
                // Validate port properties
                for prop in &port.props {
                    validate_box_prop(prop, filename)?;
                }
            }
            BoxItem::Arrow(arrow) => {
                // Validate arrow properties
                for prop in &arrow.props {
                    validate_box_prop(prop, filename)?;
                }
            }
        }
    }

    // Validate box positions if this box has a grid
    validate_box_positions(body, filename)?;

    // Validate that no two boxes have the same name
    validate_unique_box_names(body, filename)?;

    // Validate that boxes don't have both text property and child boxes
    validate_text_and_children_conflict(body, filename)?;

    Ok(())
}

/// Validate a box-level property
fn validate_box_prop(prop: &Prop, filename: &str) -> Result<(), String> {
    let span = prop.span();
    let start = span.start();
    let key = match prop {
        Prop::PropIdent { key, .. } => key,
        Prop::PropString { key, .. } => key,
        Prop::PropNumber { key, .. } => key,
        Prop::PropFrac { key, .. } => key,
        Prop::PropCoords { key, .. } => key,
        Prop::PropDim { key, .. } => key,
    };

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
            if !matches!(prop, Prop::PropDim { .. }) {
                return Err(format!("{}:{}:{}: Property 'grid' must be dimensions (heightxwidth)", filename, start.line(), start.col()));
            }
        }
        "text" => {
            if !matches!(prop, Prop::PropString { .. }) {
                return Err(format!("{}:{}:{}: Property 'text' must be a string", filename, start.line(), start.col()));
            }
        }
        "color" => {
            if let Prop::PropIdent { value, .. } = prop {
                validate_color(value, filename, span)?;
            } else {
                return Err(format!("{}:{}:{}: Property 'color' must be an identifier", filename, start.line(), start.col()));
            }
        }
        "margin" => {
            let margin_value = match prop {
                Prop::PropNumber { value, .. } => *value as f64,
                Prop::PropFrac { value, .. } => *value,
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
            if let Prop::PropIdent { value, .. } = prop {
                if !VALID_BORDER_STYLES.contains(&value.as_str()) {
                    return Err(format!(
                        "{}:{}:{}: Unknown borderStyle: '{}'. Valid styles are: {}",
                        filename,
                        start.line(),
                        start.col(),
                        value,
                        VALID_BORDER_STYLES.join(", ")
                    ));
                }
            } else {
                return Err(format!("{}:{}:{}: Property 'borderStyle' must be an identifier", filename, start.line(), start.col()));
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
        if let BoxItem::Prop(Prop::PropDim { key, value, .. }) = item {
            if key == "grid" {
                return Some(value.clone());
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

    for item in &body.items {
        if let BoxItem::BoxInst(box_inst) = item {
            let span = box_inst.span();
            let start = span.start();
            let (coords_opt, dim) = match box_inst {
                crate::ast::BoxInst::WithBody { coords, dim, .. } => (coords, dim),
                crate::ast::BoxInst::Reference { coords, dim, .. } => (coords, dim),
            };

            // Skip validation if coords is None (auto-positioning will be done during elaboration)
            let coords = match coords_opt {
                Some(c) => c,
                None => continue,
            };

            // Check if position is within grid bounds (1-based indexing)
            if coords.row < 1 || coords.row > grid_size.height {
                return Err(format!(
                    "{}:{}:{}: Box position ({}, {}) is out of bounds. Grid size is {}x{}, so row must be in range [1, {}]",
                    filename, start.line(), start.col(), coords.row, coords.col, grid_size.height, grid_size.width, grid_size.height
                ));
            }

            if coords.col < 1 || coords.col > grid_size.width {
                return Err(format!(
                    "{}:{}:{}: Box position ({}, {}) is out of bounds. Grid size is {}x{}, so col must be in range [1, {}]",
                    filename, start.line(), start.col(), coords.row, coords.col, grid_size.height, grid_size.width, grid_size.width
                ));
            }

            // Check if box with dim fits within grid bounds (1-based indexing)
            // For 1-based indexing, a box at (1, 1) with dim 1x2 occupies cells (1, 1) and (1, 2)
            let end_row = coords.row + dim.height - 1;
            let end_col = coords.col + dim.width - 1;

            if end_row > grid_size.height {
                return Err(format!(
                    "{}:{}:{}: Box at ({}, {}) with dim {}x{} extends beyond grid bounds. End row {} exceeds grid height {}",
                    filename, start.line(), start.col(), coords.row, coords.col, dim.height, dim.width, end_row, grid_size.height
                ));
            }

            if end_col > grid_size.width {
                return Err(format!(
                    "{}:{}:{}: Box at ({}, {}) with dim {}x{} extends beyond grid bounds. End col {} exceeds grid width {}",
                    filename, start.line(), start.col(), coords.row, coords.col, dim.height, dim.width, end_col, grid_size.width
                ));
            }

            // Check for overlapping cells
            for row in coords.row..=end_row {
                for col in coords.col..=end_col {
                    let cell = (row, col);
                    if occupied_cells.contains(&cell) {
                        return Err(format!(
                            "{}:{}:{}: Box at ({}, {}) with dim {}x{} overlaps with another box at cell ({}, {})",
                            filename, start.line(), start.col(), coords.row, coords.col, dim.height, dim.width, row, col
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
                crate::ast::BoxInst::WithBody { id, .. } => id.as_ref(),
                crate::ast::BoxInst::Reference { id, .. } => id.as_ref(),
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

/// Validate that boxes don't have both text property and child boxes
fn validate_text_and_children_conflict(body: &BoxBody, filename: &str) -> Result<(), String> {
    // Check if this box has a text property
    let text_prop = body.items.iter().find_map(|item| {
        if let BoxItem::Prop(Prop::PropString { key, span, .. }) = item {
            if key == "text" {
                return Some(*span);
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

    // If both exist, return an error
    if let (Some(text_span), Some(_child_span)) = (text_prop, child_box) {
        let text_start = text_span.start();
        return Err(format!(
            "{}:{}:{}: Box cannot have both 'text:' property and child boxes. Consider using a box with borderStyle: none to position a label.",
            filename,
            text_start.line(),
            text_start.col()
        ));
    }

    Ok(())
}

/// Validate that the top: property references an existing box definition
fn validate_top_property(doc: &Document, source: &str, filename: &str) -> Result<(), String> {
    // Find the top: property in diagram props
    for prop in &doc.diagram.props {
        if let Prop::PropIdent { key, value, value_location, .. } = prop {
            if key == "top" {
                // Check if a box with this name exists
                let box_exists = doc.box_defs.iter().any(|bd| bd.name == *value);
                if !box_exists {
                    // Get the span for the value to report the error at the right location
                    let value_span = crate::ast::Span::from_offsets(source, value_location.0, value_location.1);
                    let start = value_span.start();
                    return Err(format!(
                        "{}:{}:{}: No such box: {}",
                        filename,
                        start.line(),
                        start.col(),
                        value
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
                crate::ast::BoxInst::WithBody { body, .. } => {
                    // Recursively validate nested box bodies
                    validate_box_body_references(body, box_names, filename)?;
                }
                crate::ast::BoxInst::Reference { def_name, span, .. } => {
                    // Check if the referenced box exists
                    if !box_names.contains(def_name) {
                        let start = span.start();
                        return Err(format!(
                            "{}:{}:{}: No such box: {}",
                            filename,
                            start.line(),
                            start.col(),
                            def_name
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

