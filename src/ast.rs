// AST node types for the diagram language

#[derive(Debug, Clone)]
pub struct Document {
    pub version: String,
    pub diagram: Diagram,
    pub box_defs: Vec<BoxDef>,
}

#[derive(Debug, Clone)]
pub struct Diagram {
    pub props: Vec<Prop>,
}

#[derive(Debug, Clone)]
pub struct BoxDef {
    pub name: String,
    pub body: BoxBody,
}

#[derive(Debug, Clone)]
pub struct BoxBody {
    pub items: Vec<BoxItem>,
}

#[derive(Debug, Clone)]
pub enum BoxItem {
    BoxInst(BoxInst),
    Prop(Prop),
    Port(Port),
}

#[derive(Debug, Clone)]
pub enum BoxInst {
    WithBody {
        id: Option<String>,
        coords: Coords,
        dim: Dimensions,
        body: BoxBody,
    },
    Reference {
        id: Option<String>,
        coords: Coords,
        dim: Dimensions,
        def_name: String,
        location: (usize, usize), // (line, column)
    },
}

#[derive(Debug, Clone)]
pub enum Prop {
    PropIdent { key: String, value: String },
    PropString { key: String, value: Vec<String> },
    PropNumber { key: String, value: i32 },
    PropFrac { key: String, value: f64 },
    PropCoords { key: String, value: Coords },
    PropDim { key: String, value: Dimensions },
}

#[derive(Debug, Clone)]
pub struct Port {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Coords {
    pub row: i32,
    pub col: i32,
}

#[derive(Debug, Clone)]
pub struct Dimensions {
    pub height: i32,
    pub width: i32,
}
