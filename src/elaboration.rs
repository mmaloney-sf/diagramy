use std::sync::Arc;
use std::collections::HashMap;
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
    pub boxes: Vec<Box>,
}

#[derive(Debug)]
pub struct Box {
    pub def: Arc<BoxDef>,
    pub pos: (usize, usize),
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
            ast::Prop::PropIdent { key, value } if key == "color" => {
                color = value.clone();
            }
            ast::Prop::PropIdent { key, value } if key == "top" => {
                top_name = Some(value.clone());
            }
            ast::Prop::PropNumber { key, value } if key == "width" => {
                width = Some(*value as usize);
            }
            ast::Prop::PropString { key, value } if key == "title" => {
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

/// Convert an ast::BoxBody into a BoxDef, processing all items
fn convert_ast_box_body(body: &ast::BoxBody, box_def_map: &HashMap<String, &ast::BoxDef>, source: &str, filename: &str) -> Result<BoxDef, String> {
    let mut grid = (1, 1); // default grid
    let mut title: Option<String> = None;
    let mut color: Option<String> = None;
    let mut margin: Option<f64> = None;
    let mut boxes: Vec<Box> = Vec::new();

    // First pass: extract properties
    for item in &body.items {
        if let ast::BoxItem::Prop(prop) = item {
            match prop {
                ast::Prop::PropDim { key, value } if key == "grid" => {
                    grid = (value.height as usize, value.width as usize);
                }
                ast::Prop::PropString { key, value } if key == "title" || key == "text" => {
                    title = Some(value.join("\n"));
                }
                ast::Prop::PropIdent { key, value } if key == "color" => {
                    color = Some(value.clone());
                }
                ast::Prop::PropFrac { key, value } if key == "margin" => {
                    margin = Some(*value);
                }
                _ => {}
            }
        }
    }

    // Second pass: process box instances
    for item in &body.items {
        if let ast::BoxItem::BoxInst(box_inst) = item {
            match box_inst {
                ast::BoxInst::WithBody { id: _, coords, dim: _, body } => {
                    // Recursively convert the nested box body
                    let nested_def = convert_ast_box_body(body, box_def_map, source, filename)?;
                    boxes.push(Box {
                        def: Arc::new(nested_def),
                        pos: (coords.row as usize, coords.col as usize),
                    });
                }
                ast::BoxInst::Reference { id: _, coords, dim: _, def_name, location } => {
                    // Look up the referenced box definition
                    if let Some(referenced_def) = box_def_map.get(def_name) {
                        let nested_def = convert_ast_box_body(&referenced_def.body, box_def_map, source, filename)?;
                        boxes.push(Box {
                            def: Arc::new(nested_def),
                            pos: (coords.row as usize, coords.col as usize),
                        });
                    } else {
                        // Error: referenced box definition not found
                        let (line, col) = offset_to_line_col(source, location.0);
                        return Err(format!("{}:{}:{}: No such box: {}", filename, line, col, def_name));
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
        boxes,
    })
}
