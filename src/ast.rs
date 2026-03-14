// AST node types for the diagram language

// Helper enum for parsing diagram items
#[derive(Debug, Clone)]
pub enum DiagramItem {
    Prop(DiagramProperty),
    B(Box),
    P(Port),
    A(Arrow),
}

// Helper enum for parsing box children
#[derive(Debug, Clone)]
pub enum BoxChild {
    B(Box),
    P(Port),
}

#[derive(Debug, Clone)]
pub struct Document {
    pub diagram: Diagram,
    pub layout: Layout,
}

#[derive(Debug, Clone)]
pub struct Diagram {
    pub color: Option<String>,
    pub boxes: Vec<Box>,
    pub ports: Vec<Port>,
    pub arrows: Vec<Arrow>,
}

#[derive(Debug, Clone)]
pub enum DiagramProperty {
    Color(String),
}

#[derive(Debug, Clone)]
pub struct Box {
    pub id: Option<String>,  // Optional identifier after "box"
    pub properties: Vec<Property>,
    pub children: Vec<Box>,
    pub ports: Vec<Port>,
}

#[derive(Debug, Clone)]
pub enum Property {
    Title(String),
    Color(String),
    Stacked(i32),
    Vertical,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub canvas_size: Option<(i32, i32)>,  // Optional canvas size (width, height)
    pub scale: Option<f64>,                // Optional scale factor (0.0 to 1.0, from percentage)
    pub fontsize: Option<i32>,             // Optional font size
    pub items: Vec<LayoutItem>,
}

#[derive(Debug, Clone)]
pub enum LayoutEntry {
    CanvasSize(i32, i32),
    Scale(f64),
    FontSize(i32),
    BoxLayout(LayoutItem),
}

#[derive(Debug, Clone)]
pub struct LayoutItem {
    pub name: String,
    pub properties: Vec<LayoutProperty>,
}

#[derive(Debug, Clone)]
pub enum LayoutProperty {
    Pos(i32, i32),      // (x, y)
    Size(i32, i32),     // (width, height)
    Interp(i32),        // Interpolation percentage along a side
}

#[derive(Debug, Clone)]
pub struct Port {
    pub id: Option<String>,
    pub properties: Vec<PortProperty>,
}

#[derive(Debug, Clone)]
pub enum PortProperty {
    Title(String),
    Side(String),  // left, right, top, bottom
}

#[derive(Debug, Clone)]
pub struct Arrow {
    pub from: String,
    pub to: String,
}
