// AST node types for the diagram language

/// Represents a line and column position in the source file (1-indexed)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol(pub usize, pub usize);

impl LineCol {
    /// Get the line number (1-indexed)
    pub fn line(&self) -> usize {
        self.0
    }

    /// Get the column number (1-indexed)
    pub fn col(&self) -> usize {
        self.1
    }
}

/// Represents a span in the source file with start and end positions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    start: LineCol,
    end: LineCol,
}

impl Span {
    /// Create a new Span from start and end LineCol positions
    pub fn new(start: LineCol, end: LineCol) -> Self {
        Span { start, end }
    }

    /// Create a Span from byte offsets in the source text
    pub fn from_offsets(source: &str, start_offset: usize, end_offset: usize) -> Self {
        let start = offset_to_line_col(source, start_offset);
        let end = offset_to_line_col(source, end_offset);
        Span { start, end }
    }

    /// Get the start position
    pub fn start(&self) -> LineCol {
        self.start
    }

    /// Get the end position
    pub fn end(&self) -> LineCol {
        self.end
    }
}

/// Convert byte offset to line and column numbers (1-indexed)
fn offset_to_line_col(source: &str, offset: usize) -> LineCol {
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

    LineCol(line, col)
}

#[derive(Debug, Clone)]
pub struct Document {
    pub diagram: Diagram,
    pub box_defs: Vec<BoxDef>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Diagram {
    pub props: Vec<Prop>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BoxDef {
    pub name: String,
    pub name_location: (usize, usize), // byte offsets of the name identifier
    pub body: BoxBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BoxBody {
    pub items: Vec<BoxItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum BoxItem {
    BoxInst(BoxInst),
    Prop(Prop),
    Port(Port),
    Arrow(Arrow),
    Label(Label),
}

impl BoxItem {
    pub fn span(&self) -> Span {
        match self {
            BoxItem::BoxInst(inst) => inst.span(),
            BoxItem::Prop(prop) => prop.span(),
            BoxItem::Port(port) => port.span,
            BoxItem::Arrow(arrow) => arrow.span,
            BoxItem::Label(label) => label.span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum BoxInst {
    WithBody(WithBody),
    Reference(Reference),
}

#[derive(Debug, Clone)]
pub struct WithBody {
    pub id: Option<String>,
    pub coords: Option<Coords>,
    pub dim: Dim,
    pub alignment: Option<Alignment>,
    pub body: BoxBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub id: Option<String>,
    pub coords: Option<Coords>,
    pub dim: Dim,
    pub alignment: Option<Alignment>,
    pub def_name: String,
    pub location: (usize, usize), // (line, column) - deprecated, use span instead
    pub span: Span,
}

impl BoxInst {
    pub fn span(&self) -> Span {
        match self {
            BoxInst::WithBody(with_body) => with_body.span,
            BoxInst::Reference(reference) => reference.span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Prop {
    PropIdent(PropIdent), // { key: String, value: String, , span: Span },
    PropString(PropString), // { key: String, value: Vec<String>, span: Span },
    PropNumber(PropNumber), // { key: String, value: i32, span: Span },
    PropFrac(PropFrac), // { key: String, value: f64, span: Span },
    PropCoords(PropCoords), // { key: String, value: Coords, span: Span },
    PropDim(PropDim), // { key: String, value: Dim, span: Span },
}

#[derive(Debug, Clone)]
pub struct PropIdent {
    pub key: String,
    pub value: String,
    pub span: Span,
    pub value_location: (usize, usize),
}

#[derive(Debug, Clone)]
pub struct PropString {
    pub key: String,
    pub value: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct PropNumber {
    pub key: String,
    pub value: i32,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct PropFrac {
    pub key: String,
    pub value: f64,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct PropCoords {
    pub key: String,
    pub value: Coords,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct PropDim {
    pub key: String,
    pub value: Dim,
    pub span: Span,
}





impl Prop {
    pub fn span(&self) -> Span {
        match self {
            Prop::PropIdent(p) => p.span,
            Prop::PropString(p) => p.span,
            Prop::PropNumber(p) => p.span,
            Prop::PropFrac(p) => p.span,
            Prop::PropCoords(p) => p.span,
            Prop::PropDim(p) => p.span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Port {
    pub name: String,
    pub coords: Option<CoordsFrac>,  // Optional "at" clause
    pub on: Option<Side>,             // Optional "on" clause (top, bottom, left, right)
    pub alignment: Option<Alignment>, // Optional "align" clause
    pub body: Option<BoxBody>,        // Optional body (can contain labels, props, etc.)
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Arrow {
    pub from: Path,
    pub to: Path,
    pub body: Option<BoxBody>,  // Optional body (can contain labels, props, etc.)
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub text: Vec<String>,
    pub coords: Option<Coords>,
    pub dim: Option<Dim>,
    pub alignment: Option<Alignment>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alignment {
    Top,
    Right,
    Bottom,
    Left,
    Center,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Debug, Clone)]
pub struct Coords {
    pub row: i32,
    pub col: i32,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CoordsFrac {
    pub row: f64,
    pub col: f64,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Path {
    pub parts: Vec<String>,
    pub span: Span,
}

impl Path {
    pub fn to_string(&self) -> String {
        self.parts.join(".")
    }

    pub fn to_parts(&self) -> Vec<String> {
        self.parts.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Dim {
    pub height: i32,
    pub width: i32,
    pub span: Span,
}
