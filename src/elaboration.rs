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
    pub boxes: Vec<Box>,
}

#[derive(Debug)]
pub struct Box {
    pub def: Arc<BoxDef>,
    pub pos: (usize, usize),
}

/// Convert an ast::Document into a diagram::Diagram
pub fn from_ast(doc: &ast::Document) -> ElaboratedDiagram {
    // Extract diagram-level properties
    let mut color = String::from("transparent");
    let mut size = (800, 600); // default size
    let mut title: Option<String> = None;

    for prop in &doc.diagram.props {
        match prop {
            ast::Prop::PropIdent { key, value } if key == "color" => {
                color = value.clone();
            }
            ast::Prop::PropCoords { key, value } if key == "size" => {
                size = (value.x as usize, value.y as usize);
            }
            ast::Prop::PropString { key, value } if key == "title" => {
                title = Some(value.clone());
            }
            _ => {}
        }
    }

    // Build a map of box definitions for reference lookup
    let mut box_def_map: HashMap<String, &ast::BoxDef> = HashMap::new();
    for box_def in &doc.box_defs {
        box_def_map.insert(box_def.name.clone(), box_def);
    }

    // Find the "top" box definition
    // If there's a box named "top", use it; otherwise use the first box definition
    let top_ast_def = box_def_map.get("top").copied()
        .or_else(|| doc.box_defs.first())
        .expect("Document must have at least one box definition");

    // Convert the top box definition
    let top_box_def = convert_ast_box_body(&top_ast_def.body, &box_def_map);

    ElaboratedDiagram {
        color,
        size,
        title,
        top: Arc::new(top_box_def),
    }
}

/// Convert an ast::BoxBody into a BoxDef, processing all items
fn convert_ast_box_body(body: &ast::BoxBody, box_def_map: &HashMap<String, &ast::BoxDef>) -> BoxDef {
    let mut grid = (1, 1); // default grid
    let mut title: Option<String> = None;
    let mut color: Option<String> = None;
    let mut boxes: Vec<Box> = Vec::new();

    // First pass: extract properties
    for item in &body.items {
        if let ast::BoxItem::Prop(prop) = item {
            match prop {
                ast::Prop::PropCoords { key, value } if key == "grid" => {
                    grid = (value.x as usize, value.y as usize);
                }
                ast::Prop::PropString { key, value } if key == "title" || key == "text" => {
                    title = Some(value.clone());
                }
                ast::Prop::PropIdent { key, value } if key == "color" => {
                    color = Some(value.clone());
                }
                _ => {}
            }
        }
    }

    // Second pass: process box instances
    for item in &body.items {
        if let ast::BoxItem::BoxInst(box_inst) = item {
            match box_inst {
                ast::BoxInst::WithBody { id: _, coords, body } => {
                    // Recursively convert the nested box body
                    let nested_def = convert_ast_box_body(body, box_def_map);
                    boxes.push(Box {
                        def: Arc::new(nested_def),
                        pos: (coords.x as usize, coords.y as usize),
                    });
                }
                ast::BoxInst::Reference { id: _, coords, def_name } => {
                    // Look up the referenced box definition
                    if let Some(referenced_def) = box_def_map.get(def_name) {
                        let nested_def = convert_ast_box_body(&referenced_def.body, box_def_map);
                        boxes.push(Box {
                            def: Arc::new(nested_def),
                            pos: (coords.x as usize, coords.y as usize),
                        });
                    }
                }
            }
        }
    }

    BoxDef {
        grid,
        title,
        color,
        boxes,
    }
}
