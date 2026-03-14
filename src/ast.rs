// AST node types for the diagram language

#[derive(Debug, Clone)]
pub struct Document {
    pub diagram: Diagram,
    pub layout: Layout,
}

#[derive(Debug, Clone)]
pub struct Diagram {
    pub boxes: Vec<Box>,
}

#[derive(Debug, Clone)]
pub struct Box {
    pub id: Option<String>,  // Optional identifier after "box"
    pub properties: Vec<Property>,
    pub children: Vec<Box>,
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
    pub items: Vec<LayoutItem>,
}

#[derive(Debug, Clone)]
pub enum LayoutEntry {
    CanvasSize(i32, i32),
    Scale(f64),
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
}

