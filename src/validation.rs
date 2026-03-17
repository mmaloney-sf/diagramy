// Validation for the AST

use crate::ast::{Document, Prop, BoxBody, BoxItem, Coords};
use std::collections::HashSet;

// Valid colors from the color table in lib.rs
const VALID_COLORS: &[&str] = &[
    "transparent", "red", "blue", "green", "yellow", "orange", "purple", "pink",
    "cyan", "magenta", "lime", "teal", "indigo", "brown", "gray", "grey",
    "black", "white", "navy", "maroon", "olive",
];

// Valid diagram-level properties
const VALID_DIAGRAM_PROPS: &[&str] = &["size", "color", "title"];

// Valid box-level properties
const VALID_BOX_PROPS: &[&str] = &["grid", "title", "color", "text", "margin"];

/// Validate the entire document
pub fn validate(doc: &Document) -> Result<(), String> {
    // Validate diagram properties
    validate_diagram_props(&doc.diagram.props)?;

    // Validate all box definitions
    for box_def in &doc.box_defs {
        validate_box_body(&box_def.body)?;
    }

    Ok(())
}

/// Validate diagram-level properties
fn validate_diagram_props(props: &[Prop]) -> Result<(), String> {
    for prop in props {
        let key = match prop {
            Prop::PropIdent { key, .. } => key,
            Prop::PropString { key, .. } => key,
            Prop::PropNumber { key, .. } => key,
            Prop::PropFrac { key, .. } => key,
            Prop::PropCoords { key, .. } => key,
        };

        // Check if property is known
        if !VALID_DIAGRAM_PROPS.contains(&key.as_str()) {
            return Err(format!(
                "Unknown diagram property: '{}'. Valid properties are: {}",
                key,
                VALID_DIAGRAM_PROPS.join(", ")
            ));
        }

        // Validate property types
        match key.as_str() {
            "size" => {
                if !matches!(prop, Prop::PropCoords { .. }) {
                    return Err(format!("Property 'size' must be coordinates (x, y), got {:?}", prop));
                }
            }
            "color" => {
                if let Prop::PropIdent { value, .. } = prop {
                    validate_color(value)?;
                } else {
                    return Err(format!("Property 'color' must be an identifier, got {:?}", prop));
                }
            }
            "title" => {
                if !matches!(prop, Prop::PropString { .. }) {
                    return Err(format!("Property 'title' must be a string, got {:?}", prop));
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Validate box body (recursively validates nested boxes)
fn validate_box_body(body: &BoxBody) -> Result<(), String> {
    // First validate properties
    for item in &body.items {
        match item {
            BoxItem::Prop(prop) => validate_box_prop(prop)?,
            BoxItem::BoxInst(box_inst) => {
                // Recursively validate nested boxes
                match box_inst {
                    crate::ast::BoxInst::WithBody { body, .. } => {
                        validate_box_body(body)?;
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
    validate_box_positions(body)?;

    Ok(())
}

/// Validate a box-level property
fn validate_box_prop(prop: &Prop) -> Result<(), String> {
    let key = match prop {
        Prop::PropIdent { key, .. } => key,
        Prop::PropString { key, .. } => key,
        Prop::PropNumber { key, .. } => key,
        Prop::PropFrac { key, .. } => key,
        Prop::PropCoords { key, .. } => key,
    };

    // Check if property is known
    if !VALID_BOX_PROPS.contains(&key.as_str()) {
        return Err(format!(
            "Unknown box property: '{}'. Valid properties are: {}",
            key,
            VALID_BOX_PROPS.join(", ")
        ));
    }

    // Validate property types
    match key.as_str() {
        "grid" => {
            if !matches!(prop, Prop::PropCoords { .. }) {
                return Err(format!("Property 'grid' must be coordinates (x, y), got {:?}", prop));
            }
        }
        "title" | "text" => {
            if !matches!(prop, Prop::PropString { .. }) {
                return Err(format!("Property '{}' must be a string, got {:?}", key, prop));
            }
        }
        "color" => {
            if let Prop::PropIdent { value, .. } = prop {
                validate_color(value)?;
            } else {
                return Err(format!("Property 'color' must be an identifier, got {:?}", prop));
            }
        }
        "margin" => {
            if !matches!(prop, Prop::PropNumber { .. } | Prop::PropFrac { .. }) {
                return Err(format!("Property 'margin' must be a number, got {:?}", prop));
            }
        }
        _ => {}
    }

    Ok(())
}

/// Validate that a color is in the valid color table
fn validate_color(color: &str) -> Result<(), String> {
    if !VALID_COLORS.contains(&color) {
        return Err(format!(
            "Unknown color: '{}'. Valid colors are: {}",
            color,
            VALID_COLORS.join(", ")
        ));
    }
    Ok(())
}

/// Extract grid size from box properties
fn get_grid_size(body: &BoxBody) -> Option<Coords> {
    for item in &body.items {
        if let BoxItem::Prop(Prop::PropCoords { key, value }) = item {
            if key == "grid" {
                return Some(value.clone());
            }
        }
    }
    None
}

/// Validate that all child box positions are within grid bounds and unique
fn validate_box_positions(body: &BoxBody) -> Result<(), String> {
    // Get the grid size if it exists
    let grid_size = match get_grid_size(body) {
        Some(grid) => grid,
        None => return Ok(()), // No grid, no position validation needed
    };

    // Collect all box positions
    let mut positions: HashSet<(i32, i32)> = HashSet::new();

    for item in &body.items {
        if let BoxItem::BoxInst(box_inst) = item {
            let coords = match box_inst {
                crate::ast::BoxInst::WithBody { coords, .. } => coords,
                crate::ast::BoxInst::Reference { coords, .. } => coords,
            };

            // Check if position is within grid bounds
            if coords.x < 0 || coords.x >= grid_size.x {
                return Err(format!(
                    "Box position ({}, {}) is out of bounds. Grid size is ({}, {}), so x must be in range [0, {})",
                    coords.x, coords.y, grid_size.x, grid_size.y, grid_size.x
                ));
            }

            if coords.y < 0 || coords.y >= grid_size.y {
                return Err(format!(
                    "Box position ({}, {}) is out of bounds. Grid size is ({}, {}), so y must be in range [0, {})",
                    coords.x, coords.y, grid_size.x, grid_size.y, grid_size.y
                ));
            }

            // Check for duplicate positions
            let pos = (coords.x, coords.y);
            if positions.contains(&pos) {
                return Err(format!(
                    "Duplicate box position ({}, {}). Each box must have a unique position within its parent",
                    coords.x, coords.y
                ));
            }
            positions.insert(pos);
        }
    }

    Ok(())
}

