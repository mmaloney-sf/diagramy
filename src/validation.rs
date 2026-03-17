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
const VALID_DIAGRAM_PROPS: &[&str] = &["width", "color", "title", "top"];

// Valid box-level properties
const VALID_BOX_PROPS: &[&str] = &["grid", "title", "color", "text", "margin", "borderStyle"];

// Valid border styles
const VALID_BORDER_STYLES: &[&str] = &["solid", "none", "dotted", "dashed"];

/// Validate the entire document
pub fn validate(doc: &Document, _source: &str, filename: &str) -> Result<(), String> {
    // Validate diagram properties
    validate_diagram_props(&doc.diagram.props, filename)?;

    // Validate all box definitions
    for box_def in &doc.box_defs {
        validate_box_body(&box_def.body, filename)?;
    }

    Ok(())
}

/// Validate diagram-level properties
fn validate_diagram_props(props: &[Prop], filename: &str) -> Result<(), String> {
    for prop in props {
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
                "{}: Unknown diagram property: '{}'. Valid properties are: {}",
                filename,
                key,
                VALID_DIAGRAM_PROPS.join(", ")
            ));
        }

        // Validate property types
        match key.as_str() {
            "width" => {
                if !matches!(prop, Prop::PropNumber { .. }) {
                    return Err(format!("{}: Property 'width' must be a number, got {:?}", filename, prop));
                }
            }
            "color" => {
                if let Prop::PropIdent { value, .. } = prop {
                    validate_color(value, filename)?;
                } else {
                    return Err(format!("{}: Property 'color' must be an identifier, got {:?}", filename, prop));
                }
            }
            "title" => {
                if !matches!(prop, Prop::PropString { .. }) {
                    return Err(format!("{}: Property 'title' must be a string, got {:?}", filename, prop));
                }
            }
            "top" => {
                if !matches!(prop, Prop::PropIdent { .. }) {
                    return Err(format!("{}: Property 'top' must be an identifier, got {:?}", filename, prop));
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
            BoxItem::Port(_) => {
                // Ports don't have properties to validate at this level
            }
        }
    }

    // Validate box positions if this box has a grid
    validate_box_positions(body, filename)?;

    // Validate that no two boxes have the same name
    validate_unique_box_names(body, filename)?;

    Ok(())
}

/// Validate a box-level property
fn validate_box_prop(prop: &Prop, filename: &str) -> Result<(), String> {
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
            "{}: Unknown box property: '{}'. Valid properties are: {}",
            filename,
            key,
            VALID_BOX_PROPS.join(", ")
        ));
    }

    // Validate property types
    match key.as_str() {
        "grid" => {
            if !matches!(prop, Prop::PropDim { .. }) {
                return Err(format!("{}: Property 'grid' must be dimensions (heightxwidth), got {:?}", filename, prop));
            }
        }
        "title" | "text" => {
            if !matches!(prop, Prop::PropString { .. }) {
                return Err(format!("{}: Property '{}' must be a string, got {:?}", filename, key, prop));
            }
        }
        "color" => {
            if let Prop::PropIdent { value, .. } = prop {
                validate_color(value, filename)?;
            } else {
                return Err(format!("{}: Property 'color' must be an identifier, got {:?}", filename, prop));
            }
        }
        "margin" => {
            let margin_value = match prop {
                Prop::PropNumber { value, .. } => *value as f64,
                Prop::PropFrac { value, .. } => *value,
                _ => {
                    return Err(format!("{}: Property 'margin' must be a number, got {:?}", filename, prop));
                }
            };
            if margin_value < 0.0 || margin_value > 0.5 {
                return Err(format!(
                    "{}: Property 'margin' must be between 0.0 and 0.5, got {}",
                    filename,
                    margin_value
                ));
            }
        }
        "borderStyle" => {
            if let Prop::PropIdent { value, .. } = prop {
                if !VALID_BORDER_STYLES.contains(&value.as_str()) {
                    return Err(format!(
                        "{}: Unknown borderStyle: '{}'. Valid styles are: {}",
                        filename,
                        value,
                        VALID_BORDER_STYLES.join(", ")
                    ));
                }
            } else {
                return Err(format!("{}: Property 'borderStyle' must be an identifier, got {:?}", filename, prop));
            }
        }
        _ => {}
    }

    Ok(())
}

/// Validate that a color is in the valid color table
fn validate_color(color: &str, filename: &str) -> Result<(), String> {
    if !VALID_COLORS.contains(&color) {
        return Err(format!(
            "{}: Unknown color: '{}'. Valid colors are: {}",
            filename,
            color,
            VALID_COLORS.join(", ")
        ));
    }
    Ok(())
}

/// Extract grid size from box properties
fn get_grid_size(body: &BoxBody) -> Option<crate::ast::Dimensions> {
    for item in &body.items {
        if let BoxItem::Prop(Prop::PropDim { key, value }) = item {
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
            let (coords, dim) = match box_inst {
                crate::ast::BoxInst::WithBody { coords, dim, .. } => (coords, dim),
                crate::ast::BoxInst::Reference { coords, dim, .. } => (coords, dim),
            };

            // Check if position is within grid bounds (1-based indexing)
            if coords.row < 1 || coords.row > grid_size.height {
                return Err(format!(
                    "{}: Box position ({}, {}) is out of bounds. Grid size is {}x{}, so row must be in range [1, {}]",
                    filename, coords.row, coords.col, grid_size.height, grid_size.width, grid_size.height
                ));
            }

            if coords.col < 1 || coords.col > grid_size.width {
                return Err(format!(
                    "{}: Box position ({}, {}) is out of bounds. Grid size is {}x{}, so col must be in range [1, {}]",
                    filename, coords.row, coords.col, grid_size.height, grid_size.width, grid_size.width
                ));
            }

            // Check if box with dim fits within grid bounds (1-based indexing)
            // For 1-based indexing, a box at (1, 1) with dim 1x2 occupies cells (1, 1) and (1, 2)
            let end_row = coords.row + dim.height - 1;
            let end_col = coords.col + dim.width - 1;

            if end_row > grid_size.height {
                return Err(format!(
                    "{}: Box at ({}, {}) with dim {}x{} extends beyond grid bounds. End row {} exceeds grid height {}",
                    filename, coords.row, coords.col, dim.height, dim.width, end_row, grid_size.height
                ));
            }

            if end_col > grid_size.width {
                return Err(format!(
                    "{}: Box at ({}, {}) with dim {}x{} extends beyond grid bounds. End col {} exceeds grid width {}",
                    filename, coords.row, coords.col, dim.height, dim.width, end_col, grid_size.width
                ));
            }

            // Check for overlapping cells
            for row in coords.row..=end_row {
                for col in coords.col..=end_col {
                    let cell = (row, col);
                    if occupied_cells.contains(&cell) {
                        return Err(format!(
                            "{}: Box at ({}, {}) with dim {}x{} overlaps with another box at cell ({}, {})",
                            filename, coords.row, coords.col, dim.height, dim.width, row, col
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
            let id = match box_inst {
                crate::ast::BoxInst::WithBody { id, .. } => id.as_ref(),
                crate::ast::BoxInst::Reference { id, .. } => id.as_ref(),
            };

            // Only check named boxes
            if let Some(name) = id {
                if names.contains(name) {
                    return Err(format!(
                        "{}: Duplicate box name '{}'. Each box must have a unique name within its parent",
                        filename, name
                    ));
                }
                names.insert(name.clone());
            }
        }
    }

    Ok(())
}

