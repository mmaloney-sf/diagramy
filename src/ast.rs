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
    pub properties: Vec<Property>,
    pub children: Vec<Box>,
}

#[derive(Debug, Clone)]
pub enum Property {
    Title(String),
    Color(String),
    Stack(i32),
    Vertical,
}

#[derive(Debug, Clone)]
pub struct Layout {
    // Layout properties can be added later
}

