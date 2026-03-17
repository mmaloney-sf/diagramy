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
    pub version: String,
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
}

impl BoxItem {
    pub fn span(&self) -> Span {
        match self {
            BoxItem::BoxInst(inst) => inst.span(),
            BoxItem::Prop(prop) => prop.span(),
            BoxItem::Port(port) => port.span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum BoxInst {
    WithBody {
        id: Option<String>,
        coords: Option<Coords>,
        dim: Dim,
        body: BoxBody,
        span: Span,
    },
    Reference {
        id: Option<String>,
        coords: Option<Coords>,
        dim: Dim,
        def_name: String,
        location: (usize, usize), // (line, column) - deprecated, use span instead
        span: Span,
    },
}

impl BoxInst {
    pub fn span(&self) -> Span {
        match self {
            BoxInst::WithBody { span, .. } => *span,
            BoxInst::Reference { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Prop {
    PropIdent { key: String, value: String, span: Span },
    PropString { key: String, value: Vec<String>, span: Span },
    PropNumber { key: String, value: i32, span: Span },
    PropFrac { key: String, value: f64, span: Span },
    PropCoords { key: String, value: Coords, span: Span },
    PropDim { key: String, value: Dim, span: Span },
}

impl Prop {
    pub fn span(&self) -> Span {
        match self {
            Prop::PropIdent { span, .. } => *span,
            Prop::PropString { span, .. } => *span,
            Prop::PropNumber { span, .. } => *span,
            Prop::PropFrac { span, .. } => *span,
            Prop::PropCoords { span, .. } => *span,
            Prop::PropDim { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Port {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Coords {
    pub row: i32,
    pub col: i32,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Dim {
    pub height: i32,
    pub width: i32,
    pub span: Span,
}
