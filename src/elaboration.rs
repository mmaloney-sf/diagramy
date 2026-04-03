use crate::ast;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug)]
pub struct ElaboratedDiagram {
    pub color: String,
    pub size: (usize, usize),
    pub title: Option<String>,
    pub cheat_ports: bool,
    pub debug: bool,
    pub top: Arc<BoxDef>,
}

#[derive(Debug)]
pub struct BoxDef {
    pub grid: (usize, usize),
    pub title: Option<String>,
    pub color: Option<String>,
    pub margin: Option<f64>,
    pub border_style: Option<String>,
    pub bold: Option<bool>,
    pub debug: Option<bool>,
    pub def_name: Option<String>, // Name of the box definition (None for inline boxes)
    pub line_number: Option<usize>, // Line number where the box was defined
    pub boxes: Vec<BoxInst>,
    pub ports: Vec<Port>,
    pub arrows: Vec<Arrow>,
    pub routed_arrow_paths: Vec<RoutedArrowPath>, // Routed paths in fractional coordinates
    pub kind: BoxKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoxKind {
    Box,
    Label,
    Group,
}

#[derive(Clone)]
pub struct RoutedArrowPath {
    path: Vec<(f64, f64)>,
}

impl RoutedArrowPath {
    pub fn new(path: Vec<(f64, f64)>) -> Self {
        Self { path }
    }

    pub fn iter(&self) -> impl Iterator<Item = &(f64, f64)> {
        self.path.iter()
    }
}

#[derive(Debug, Clone)]
pub struct Port {
    pub name: String,
    pub coords: (f64, f64), // Fractional coordinates
    pub label: Option<String>, // Optional label text
    pub used_at_clause: bool, // True if positioned with "at", false if positioned with "on"
}

#[derive(Debug, Clone)]
pub struct Arrow {
    pub from: String,
    pub to: String,
}

#[derive(Debug)]
pub struct BoxInst {
    pub id: Option<String>, // Optional identifier for the box instance
    pub def: Arc<BoxDef>,
    pub pos: (usize, usize),
    pub dim: (usize, usize), // (height, width) - number of grid cells to span
    pub alignment: ast::Alignment, // Alignment within the grid cell (defaults to Center)
    pub debug: bool,
}

#[derive(Debug)]
struct Elaborator<'ast> {
    filename: String,
    debug_dir: Option<String>,
    cheat_ports: bool,
    box_def_map: HashMap<String, &'ast ast::BoxDef>,
}

/// Convert an ast::Document into a diagram::Diagram
pub fn from_ast(
    doc: &ast::Document,
    filename: &str,
    debug_dir: Option<&str>,
) -> Result<ElaboratedDiagram, String> {
    // Build a map of box definitions for reference lookup
    let mut box_def_map: HashMap<String, &ast::BoxDef> = HashMap::new();
    for box_def in &doc.box_defs {
        box_def_map.insert(box_def.name.clone(), box_def);
    }

    let mut elaborator = Elaborator {
        filename: filename.to_string(),
        debug_dir: debug_dir.map(|s| s.to_string()),
        cheat_ports: false, // Will be set from diagram properties
        box_def_map,
    };

    elaborator.from_ast(doc)
}

impl<'ast> Elaborator<'ast> {
    /// Convert an ast::Document into a diagram::Diagram
    fn from_ast(
        &mut self,
        doc: &ast::Document,
    ) -> Result<ElaboratedDiagram, String> {
        // Extract diagram-level properties
        let mut color = String::from("transparent");
        let mut width: Option<usize> = None;
        let mut title: Option<String> = None;
        let mut top_name: Option<String> = None;
        let mut cheat_ports = false;
        let mut diagram_debug: Option<bool> = None;

        for prop in &doc.diagram.props {
            match prop {
                ast::Prop::PropIdent(p) if p.key == "color" => {
                    color = p.value.clone();
                }
                ast::Prop::PropIdent(p) if p.key == "top" => {
                    top_name = Some(p.value.clone());
                }
                ast::Prop::PropIdent(p) if p.key == "cheatPorts" => {
                    cheat_ports = p.value == "true";
                    self.cheat_ports = cheat_ports;
                }
                ast::Prop::PropIdent(p) if p.key == "debug" => {
                    diagram_debug = Some(p.value == "true");
                }
                ast::Prop::PropNumber(p) if p.key == "width" => {
                    width = Some(p.value as usize);
                }
                ast::Prop::PropString(p) if p.key == "title" => {
                    title = Some(p.value.join("\n"));
                }
                _ => {}
            }
        }

        // Find the top box definition based on the following priority:
        // 1. If "top" property is specified in diagram section, use that BoxDef
        // 2. Otherwise, use the first box definition
        // 3. If no box definitions exist, error out
        let top_ast_def = if let Some(ref name) = top_name {
            // top: property was specified, look it up
            self.box_def_map
                .get(name)
                .copied()
                .ok_or_else(|| format!("{}:0:0: No such box: {}", self.filename, name))?
        } else {
            // No top: property, use first box
            doc.box_defs.first().ok_or_else(|| {
                format!(
                    "{}:0:0: Document must have at least one box definition",
                    self.filename
                )
            })?
        };

        // Convert the top box definition
        let top_def_name = Some(top_ast_def.name.clone());
        let top_line_number = Some(top_ast_def.span.start().line());
        let mut top_box_def = self.convert_ast_box_body(&top_ast_def.body, "top", top_def_name, top_line_number)?;

        // Apply diagram-level debug property to the top box if specified
        if let Some(debug) = diagram_debug {
            top_box_def.debug = Some(debug);
        }

        // Calculate size from width and grid aspect ratio
        // grid is now (rows, cols), so aspect_ratio = rows / cols
        let width = width.unwrap_or(800); // default width
        let (grid_rows, grid_cols) = top_box_def.grid;
        let aspect_ratio = grid_rows as f64 / grid_cols as f64;
        let height = (width as f64 * aspect_ratio) as usize;
        let size = (width, height);

        Ok(dbg!(ElaboratedDiagram {
            color,
            size,
            title,
            cheat_ports,
            debug: diagram_debug.unwrap_or(false),
            top: Arc::new(top_box_def),
        }))
    }

    /// Process an inline box instance (WithBody variant)
    fn process_inline_box(
        &mut self,
        with_body: &ast::WithBody,
        box_name: &str,
        grid: (usize, usize),
        occupied: &mut HashSet<(i32, i32)>,
        last_pos: &mut (i32, i32),
    ) -> Result<BoxInst, String> {
        // First, process the body to get the grid so we can determine default dim
        let line_number = Some(with_body.span.start().line());
        let nested_def = self.convert_ast_box_body(
            &with_body.body,
            &format!("{}.inline", box_name),
            None, // No def_name for inline boxes
            line_number,
        )?;

        // If dim is not specified, default to the child's grid
        let dim = if let Some(ref d) = with_body.dim {
            (d.height, d.width)
        } else {
            (nested_def.grid.0 as i32, nested_def.grid.1 as i32)
        };

        // Determine position (auto-position if coords is None)
        let (row, col) = if let Some(c) = &with_body.coords {
            (c.row, c.col)
        } else {
            match self.find_next_free_position(
                occupied,
                grid,
                dim,
                *last_pos,
            ) {
                Some(pos) => pos,
                None => {
                    let start = with_body.span.start();
                    return Err(format!(
                        "{}:{}:{}: Cannot auto-position box with dim {}x{}. No free space available in {}x{} grid",
                        self.filename, start.line(), start.col(), dim.0, dim.1, grid.0, grid.1
                    ));
                }
            }
        };

        // Update last position
        *last_pos = (row, col);

        // Check for overlaps and mark occupied cells (including cells occupied by dim)
        for r in row..(row + dim.0) {
            for c in col..(col + dim.1) {
                if occupied.contains(&(r, c)) {
                    let start = with_body.span.start();
                    return Err(format!(
                        "{}:{}:{}: Box at ({}, {}) with dim {}x{} overlaps with another box at cell ({}, {})",
                        self.filename, start.line(), start.col(), row, col, dim.0, dim.1, r, c
                    ));
                }
                occupied.insert((r, c));
            }
        }

        let debug = nested_def.debug.unwrap_or(false);
        let def = Arc::new(nested_def);

        Ok(BoxInst {
            id: with_body.id.clone(),
            def,
            // Convert from 1-based to 0-based indexing
            pos: ((row - 1) as usize, (col - 1) as usize),
            dim: (dim.0 as usize, dim.1 as usize),
            alignment: with_body.alignment.clone().unwrap_or(ast::Alignment::Center),
            debug,
        })
    }

    /// Process a box reference (Reference variant)
    fn process_box_reference(
        &mut self,
        reference: &ast::Reference,
        grid: (usize, usize),
        occupied: &mut HashSet<(i32, i32)>,
        last_pos: &mut (i32, i32),
    ) -> Result<BoxInst, String> {
        // Look up the referenced box definition first to get its grid
        let referenced_def = self.box_def_map.get(&reference.def_name).ok_or_else(|| {
            let start = reference.span.start();
            format!(
                "{}:{}:{}: No such box: {}",
                self.filename,
                start.line(),
                start.col(),
                reference.def_name
            )
        })?;

        let def_name = Some(referenced_def.name.clone());
        let line_number = Some(referenced_def.span.start().line());
        let nested_def = self.convert_ast_box_body(
            &referenced_def.body,
            &reference.def_name,
            def_name,
            line_number,
        )?;

        // If dim is not specified, default to the referenced box's grid
        let dim = if let Some(ref d) = reference.dim {
            (d.height, d.width)
        } else {
            (nested_def.grid.0 as i32, nested_def.grid.1 as i32)
        };

        // Determine position (auto-position if coords is None)
        let (row, col) = if let Some(c) = &reference.coords {
            (c.row, c.col)
        } else {
            match self.find_next_free_position(
                occupied,
                grid,
                dim,
                *last_pos,
            ) {
                Some(pos) => pos,
                None => {
                    let start = reference.span.start();
                    return Err(format!(
                        "{}:{}:{}: Cannot auto-position box '{}' with dim {}x{}. No free space available in {}x{} grid",
                        self.filename, start.line(), start.col(), reference.def_name, dim.0, dim.1, grid.0, grid.1
                    ));
                }
            }
        };

        // Update last position
        *last_pos = (row, col);

        // Check for overlaps and mark occupied cells (including cells occupied by dim)
        for r in row..(row + dim.0) {
            for c in col..(col + dim.1) {
                if occupied.contains(&(r, c)) {
                    let start = reference.span.start();
                    return Err(format!(
                        "{}:{}:{}: Box at ({}, {}) with dim {}x{} overlaps with another box at cell ({}, {})",
                        self.filename, start.line(), start.col(), row, col, dim.0, dim.1, r, c
                    ));
                }
                occupied.insert((r, c));
            }
        }

        let debug = nested_def.debug.unwrap_or(false);
        let def = Arc::new(nested_def);

        Ok(BoxInst {
            id: reference.id.clone(),
            def,
            // Convert from 1-based to 0-based indexing
            pos: ((row - 1) as usize, (col - 1) as usize),
            dim: (dim.0 as usize, dim.1 as usize),
            alignment: reference.alignment.clone().unwrap_or(ast::Alignment::Center),
            debug,
        })
    }

    /// Process a label (converts to a box with text property)
    fn process_label(
        &mut self,
        label: &ast::Label,
        grid: (usize, usize),
        occupied: &mut HashSet<(i32, i32)>,
        last_pos: &mut (i32, i32),
    ) -> Result<BoxInst, String> {
        // Get dimensions from label, defaulting to 1x1
        let (dim_height, dim_width) = if let Some(ref dim) = label.dim {
            (dim.height, dim.width)
        } else {
            (1, 1)
        };

        // Determine position (auto-position if coords is None)
        let (row, col) = if let Some(c) = &label.coords {
            (c.row, c.col)
        } else {
            match self.find_next_free_position(
                occupied,
                grid,
                (dim_height, dim_width),
                *last_pos,
            ) {
                Some(pos) => pos,
                None => {
                    let start = label.span.start();
                    return Err(format!(
                        "{}:{}:{}: Cannot auto-position label with dim {}x{}. No free space available in {}x{} grid",
                        self.filename, start.line(), start.col(), dim_height, dim_width, grid.0, grid.1
                    ));
                }
            }
        };

        // Update last position
        *last_pos = (row, col);

        // Check for overlaps and mark all occupied cells
        for r in row..(row + dim_height as i32) {
            for c in col..(col + dim_width as i32) {
                if occupied.contains(&(r, c)) {
                    let start = label.span.start();
                    return Err(format!(
                        "{}:{}:{}: Label at ({}, {}) with dim {}x{} overlaps with another box at ({}, {})",
                        self.filename, start.line(), start.col(), row, col, dim_height, dim_width, r, c
                    ));
                }
                occupied.insert((r, c));
            }
        }

        // Create a box definition with the text property
        // The label text becomes the title (which is rendered as text in the box)
        // Multi-line labels are joined with newlines
        let box_def = BoxDef {
            grid: (1, 1),
            title: Some(label.text.join("\n")),
            color: None,
            margin: None,
            border_style: Some("none".to_string()), // Labels have no border by default
            bold: None,
            debug: None,
            def_name: None, // Labels don't have def names
            line_number: Some(label.span.start().line()),
            boxes: Vec::new(),
            ports: Vec::new(),
            arrows: Vec::new(),
            routed_arrow_paths: Vec::new(),
            kind: BoxKind::Label,
        };

        Ok(BoxInst {
            id: None, // Labels don't have IDs
            def: Arc::new(box_def),
            // Convert from 1-based to 0-based indexing
            pos: ((row - 1) as usize, (col - 1) as usize),
            dim: (dim_height as usize, dim_width as usize),
            alignment: label.alignment.clone().unwrap_or(ast::Alignment::Center),
            debug: false, // Labels default to no debug
        })
    }

    /// Process ports from AST, handling both "at" and "on" clauses
    fn process_ports(
        &mut self,
        body: &ast::BoxBody,
        grid: (usize, usize),
    ) -> Result<Vec<Port>, String> {
        let mut ports = Vec::new();
        let mut used_positions: HashSet<(i32, i32)> = HashSet::new(); // Track used edge positions

        for item in &body.items {
            if let ast::BoxItem::Port(port) = item {
                let (coords, used_at_clause) = if let Some(ref coords_frac) = port.coords {
                    // Explicit "at" positioning - shift half a grid cell up and to the left
                    ((coords_frac.row - 0.5, coords_frac.col - 0.5), true)
                } else {
                    // Use "on" clause (or default to "right")
                    let side = match &port.on {
                        Some(ast::Side::Top) => "top",
                        Some(ast::Side::Right) => "right",
                        Some(ast::Side::Bottom) => "bottom",
                        Some(ast::Side::Left) => "left",
                        None => "right",
                    };
                    (self.find_port_position_on_side(side, grid, &mut used_positions, port)?, false)
                };

                // Extract label from port body if present
                let label = if let Some(ref port_body) = port.body {
                    self.extract_label_from_body(port_body)
                } else {
                    None
                };

                ports.push(Port {
                    name: port.name.clone(),
                    coords,
                    label,
                    used_at_clause,
                });
            }
        }

        Ok(ports)
    }

    /// Extract label text from a body (port or arrow)
    /// Returns the first label found, or None if no labels exist
    fn extract_label_from_body(&self, body: &ast::BoxBody) -> Option<String> {
        for item in &body.items {
            if let ast::BoxItem::Label(label) = item {
                // Join multi-line labels with newlines
                return Some(label.text.join("\n"));
            }
        }
        None
    }

    /// Find a free position for a port on the given side of the grid
    fn find_port_position_on_side(
        &self,
        side: &str,
        grid: (usize, usize),
        used_positions: &mut HashSet<(i32, i32)>,
        port: &ast::Port,
    ) -> Result<(f64, f64), String> {
        let (grid_height, grid_width) = grid;
        let height = grid_height as f64;
        let width = grid_width as f64;

        // Try to find a free position on the specified side
        // We'll use integer positions to track usage, but return fractional coordinates
        match side {
            "left" => {
                // Find free position on left side: (row, 0.0)
                for r in 1..=grid_height {
                    if !used_positions.contains(&(r as i32, 0)) {
                        used_positions.insert((r as i32, 0));
                        return Ok(((r as f64 - 0.5), 0.0));
                    }
                }
                let start = port.span.start();
                return Err(format!(
                    "{}:{}:{}: No free position on left side for port '{}'",
                    self.filename, start.line(), start.col(), port.name
                ));
            }
            "right" => {
                // Find free position on right side: (row, WIDTH)
                for r in 1..=grid_height {
                    if !used_positions.contains(&(r as i32, grid_width as i32 + 1)) {
                        used_positions.insert((r as i32, grid_width as i32 + 1));
                        return Ok(((r as f64 - 0.5), width));
                    }
                }
                let start = port.span.start();
                return Err(format!(
                    "{}:{}:{}: No free position on right side for port '{}'",
                    self.filename, start.line(), start.col(), port.name
                ));
            }
            "top" => {
                // Find free position on top side: (0.0, col)
                for c in 1..=grid_width {
                    if !used_positions.contains(&(0, c as i32)) {
                        used_positions.insert((0, c as i32));
                        return Ok((0.0, (c as f64 - 0.5)));
                    }
                }
                let start = port.span.start();
                return Err(format!(
                    "{}:{}:{}: No free position on top side for port '{}'",
                    self.filename, start.line(), start.col(), port.name
                ));
            }
            "bottom" => {
                // Find free position on bottom side: (HEIGHT, col)
                for c in 1..=grid_width {
                    if !used_positions.contains(&(grid_height as i32 + 1, c as i32)) {
                        used_positions.insert((grid_height as i32 + 1, c as i32));
                        return Ok((height, (c as f64 - 0.5)));
                    }
                }
                let start = port.span.start();
                return Err(format!(
                    "{}:{}:{}: No free position on bottom side for port '{}'",
                    self.filename, start.line(), start.col(), port.name
                ));
            }
            _ => {
                let start = port.span.start();
                return Err(format!(
                    "{}:{}:{}: Invalid 'on' value '{}' for port '{}'. Must be one of: top, bottom, left, right",
                    self.filename, start.line(), start.col(), side, port.name
                ));
            }
        }
    }

    /// Extract properties, ports, and arrows from a box body
    /// Returns (grid, title, color, margin, border_style, bold, debug, arrows)
    /// Note: ports are now processed separately after grid is known
    fn extract_box_items(
        &mut self,
        body: &ast::BoxBody,
    ) -> (
        (usize, usize),
        Option<String>,
        Option<String>,
        Option<f64>,
        Option<String>,
        Option<bool>,
        Option<bool>,
        Vec<Arrow>,
    ) {
        let mut grid: Option<(usize, usize)> = None; // Will be calculated if not specified
        let mut title: Option<String> = None;
        let mut color: Option<String> = None;
        let mut margin: Option<f64> = None;
        let mut border_style: Option<String> = None;
        let mut bold: Option<bool> = None;
        let mut debug: Option<bool> = None;
        let mut arrows: Vec<Arrow> = Vec::new();
        let mut child_count = 0;

        for item in &body.items {
            match item {
                ast::BoxItem::Prop(prop) => match prop {
                    ast::Prop::PropDim(p) if p.key == "grid" => {
                        grid = Some((p.value.height as usize, p.value.width as usize));
                    }
                    ast::Prop::PropString(p) if p.key == "text" => {
                        title = Some(p.value.join("\n"));
                    }
                    ast::Prop::PropIdent(p) if p.key == "color" => {
                        color = Some(p.value.clone());
                    }
                    ast::Prop::PropIdent(p) if p.key == "borderStyle" => {
                        border_style = Some(p.value.clone());
                    }
                    ast::Prop::PropIdent(p) if p.key == "bold" => {
                        bold = Some(p.value == "true");
                    }
                    ast::Prop::PropIdent(p) if p.key == "debug" => {
                        debug = Some(p.value == "true");
                    }
                    ast::Prop::PropFrac(p) if p.key == "margin" => {
                        margin = Some(p.value);
                    }
                    _ => {}
                },
                ast::BoxItem::Port(_port) => {
                    // Port positioning will be handled later after we know the grid size
                    // For now, we'll store ports and process them after extracting all items
                    // This is a placeholder - we'll process ports separately
                }
                ast::BoxItem::Arrow(arrow) => {
                    arrows.push(Arrow {
                        from: arrow.from.to_string(),
                        to: arrow.to.to_string(),
                    });
                }
                ast::BoxItem::BoxInst(_) | ast::BoxItem::Label(_) => {
                    child_count += 1;
                }
            }
        }

        // Calculate default grid if not specified: NxN where N = ceil(sqrt(# children))
        let final_grid = grid.unwrap_or_else(|| {
            if child_count == 0 {
                (1, 1) // No children, default to 1x1
            } else {
                let n = (child_count as f64).sqrt().ceil() as usize;
                (n, n)
            }
        });

        (final_grid, title, color, margin, border_style, bold, debug, arrows)
    }

    /// Find the next free grid position that can fit a box with the given dimensions
    /// Starts scanning from the position FOLLOWING last_pos
    /// Returns Some((row, col)) in 1-based indexing, or None if no position found
    fn find_next_free_position(
        &mut self,
        occupied: &HashSet<(i32, i32)>,
        grid: (usize, usize),
        dim: (i32, i32),
        last_pos: (i32, i32),
    ) -> Option<(i32, i32)> {
        let (grid_rows, grid_cols) = grid;
        let (dim_height, dim_width) = dim;
        let (last_row, last_col) = last_pos;

        // Calculate the starting position (next position after last_pos)
        let (start_row, start_col) = if last_col >= grid_cols as i32 {
            (last_row + 1, 1)
        } else {
            (last_row, last_col + 1)
        };

        // Scan from start position to end of grid
        for row in start_row..=(grid_rows as i32) {
            let col_start = if row == start_row { start_col } else { 1 };
            for col in col_start..=(grid_cols as i32) {
                // Check if the box would fit within grid bounds
                let end_row = row + dim_height - 1;
                let end_col = col + dim_width - 1;

                if end_row > grid_rows as i32 || end_col > grid_cols as i32 {
                    continue; // Box doesn't fit within grid bounds at this position
                }

                // Check if all cells needed by this box are free
                let mut all_free = true;
                for r in row..=end_row {
                    for c in col..=end_col {
                        if occupied.contains(&(r, c)) {
                            all_free = false;
                            break;
                        }
                    }
                    if !all_free {
                        break;
                    }
                }

                if all_free {
                    return Some((row, col));
                }
            }
        }

        // If no free position found, return None
        None
    }

    /// Convert an ast::BoxBody into a BoxDef, processing all items
    fn convert_ast_box_body(
        &mut self,
        body: &ast::BoxBody,
        box_name: &str,
        def_name: Option<String>,
        line_number: Option<usize>,
    ) -> Result<BoxDef, String> {
        // First pass: extract properties and arrows
        let (grid, title, color, margin, border_style, bold, debug, arrows) = self.extract_box_items(body);

        // Process ports after we know the grid size
        let ports = self.process_ports(body, grid)?;

        let mut boxes: Vec<BoxInst> = Vec::new();

        // Second pass: process box instances with auto-positioning
        // Track occupied grid cells for auto-positioning
        let mut occupied: HashSet<(i32, i32)> = HashSet::new();
        // Track the last position for auto-positioning
        let mut last_pos = (1, 0); // Start before (1, 1)

        for item in &body.items {
            match item {
                ast::BoxItem::BoxInst(box_inst) => {
                    match box_inst {
                        ast::BoxInst::WithBody(with_body) => {
                            let box_def = self.process_inline_box(
                                with_body,
                                box_name,
                                grid,
                                &mut occupied,
                                &mut last_pos,
                            )?;
                            boxes.push(box_def);
                        }
                        ast::BoxInst::Reference(reference) => {
                            let box_def = self.process_box_reference(
                                reference,
                                grid,
                                &mut occupied,
                                &mut last_pos,
                            )?;
                            boxes.push(box_def);
                        }
                    }
                }
                ast::BoxItem::Label(label) => {
                    let box_def = self.process_label(
                        label,
                        grid,
                        &mut occupied,
                        &mut last_pos,
                    )?;
                    boxes.push(box_def);
                }
                _ => {}
            }
        }

        // Route arrows using A* pathfinding
        let routed_arrow_paths =
            self.route_arrows(&arrows, &ports, &boxes, grid, margin, box_name);

        Ok(BoxDef {
            grid,
            title,
            color,
            margin,
            border_style,
            bold,
            debug,
            def_name,
            line_number,
            boxes,
            ports,
            arrows,
            routed_arrow_paths,
            kind: BoxKind::Box,
        })
    }

    /// Route arrows using A* pathfinding
    fn route_arrows(
        &mut self,
        arrows: &[Arrow],
        ports: &[Port],
        boxes: &[BoxInst],
        grid: (usize, usize),
        parent_margin: Option<f64>,
        box_name: &str,
    ) -> Vec<RoutedArrowPath> {
        use crate::routing::{ArrowRouter, BoundingBox};

        // Build port map (using f64 coordinates)
        let mut port_map: HashMap<String, (f64, f64)> = HashMap::new();

        // Add ports from the current box
        for port in ports {
            port_map.insert(port.name.clone(), (port.coords.0, port.coords.1));
        }

        // Add ports from immediate child boxes (with qualified names like "childbox.portname")
        for child_box in boxes {
            if let Some(ref child_id) = child_box.id {
                // Get child box position and dimensions
                let (child_row, child_col) = child_box.pos;
                let (child_height, child_width) = child_box.dim;

                // Add each port from the child box with qualified name
                for child_port in &child_box.def.ports {
                    // Calculate the port's position in the parent's coordinate system
                    // Child box occupies cells from (child_row, child_col) to (child_row + child_height, child_col + child_width)
                    // Port coordinates are relative to the child box's grid
                    // We need to scale and offset them to the parent's coordinate system

                    // IMPORTANT: Account for the margin that will be applied to the child box during rendering
                    // The margin is based on cell size, not box size (see diagram.rs flatten_boxes)
                    // Margin is applied as: margin_x = cell_width * margin_factor
                    // This shrinks the child box, so ports need to be adjusted accordingly

                    let margin_factor = parent_margin.unwrap_or(0.1);

                    // Calculate margin in parent grid cells
                    // Each cell is 1.0 units in the parent grid
                    let margin_in_cells = margin_factor;

                    // The child box's effective position and size after margin
                    let effective_child_row = (child_row as f64) + margin_in_cells;
                    let effective_child_col = (child_col as f64) + margin_in_cells;
                    let effective_child_height = (child_height as f64) - (2.0 * margin_in_cells);
                    let effective_child_width = (child_width as f64) - (2.0 * margin_in_cells);

                    let child_grid = child_box.def.grid;
                    let (child_grid_rows, child_grid_cols) = child_grid;

                    // Port coordinates in child's fractional grid coordinates
                    let (port_row_in_child, port_col_in_child) = child_port.coords;

                    // Scale port coordinates from child's grid to child's effective cell span (after margin)
                    let port_row_in_cells = port_row_in_child * effective_child_height / (child_grid_rows as f64);
                    let port_col_in_cells = port_col_in_child * effective_child_width / (child_grid_cols as f64);

                    // Offset by child box's effective position in parent grid (after margin)
                    let port_row_in_parent = effective_child_row + port_row_in_cells;
                    let port_col_in_parent = effective_child_col + port_col_in_cells;

                    // Add to port map with qualified name
                    let qualified_name = format!("{}.{}", child_id, child_port.name);
                    port_map.insert(qualified_name, (port_row_in_parent, port_col_in_parent));
                }
            }
        }

        // Build bounding boxes for child boxes
        let mut bounding_boxes = Vec::new();
        for child_box in boxes {
            let (row, col) = child_box.pos;
            let (height, width) = child_box.dim;

            // Box at position (row, col) with dimensions (height, width)
            // Note: pos is 0-based indexing

            // Trim GRID_RESOLUTION boxes off the edges of all obstructions
            // GRID_RESOLUTION = 10 in discretized space = 1.0 in fractional space
            let trim = 1.0;

            let min_row = row as f64 + trim;
            let min_col = col as f64 + trim;
            let max_row = (row + height) as f64 - trim;
            let max_col = (col + width) as f64 - trim;

            // Store bounding box in fractional coordinates
            // The ArrowRouter will scale these by its grid_resolution
            bounding_boxes.push(BoundingBox {
                min_frac: (min_row, min_col),
                max_frac: (max_row, max_col),
            });
        }

        // Create router
        let mut router = ArrowRouter::new(
            grid.1 as f64, // grid width
            grid.0 as f64, // grid height
            bounding_boxes,
        );

        // Set debug directory if provided
        if let Some(dir) = &self.debug_dir {
            router.set_debug_dir(dir, box_name);
        }

        // Get grid resolution from router
        let grid_resolution = router.grid_resolution();

        // Route each arrow
        let mut routed_paths = Vec::new();
        for arrow in arrows {
            if let (Some(&start), Some(&end)) = (port_map.get(&arrow.from), port_map.get(&arrow.to)) {
                // Check if cheatPorts is enabled and either start or end is a subport
                let is_start_subport = arrow.from.contains('.');
                let is_end_subport = arrow.to.contains('.');

                if self.cheat_ports && (is_start_subport || is_end_subport) {
                    // Skip routing and use fallback straight line
                    routed_paths.push(RoutedArrowPath::new(vec![start, end]));
                    continue;
                }

                // Determine which child boxes (if any) contain the start and end ports
                let mut excluded_box_indices = Vec::new();

                // Check if start port is in a child box
                if is_start_subport {
                    let child_id = arrow.from.split('.').next().unwrap();
                    if let Some(idx) = boxes.iter().position(|b| b.id.as_deref() == Some(child_id)) {
                        excluded_box_indices.push(idx);
                    }
                }

                // Check if end port is in a child box
                if is_end_subport {
                    let child_id = arrow.to.split('.').next().unwrap();
                    if let Some(idx) = boxes.iter().position(|b| b.id.as_deref() == Some(child_id)) {
                        if !excluded_box_indices.contains(&idx) {
                            excluded_box_indices.push(idx);
                        }
                    }
                }

                if let Some(path) = router.route(start, end, &excluded_box_indices) {
                    // Convert i32 points back to f64 for storage
                    let f64_points: Vec<(f64, f64)> = path
                        .points
                        .iter()
                        .map(|(row, col)| {
                            (
                                *row as f64 / grid_resolution as f64,
                                *col as f64 / grid_resolution as f64,
                            )
                        })
                        .collect();
                    routed_paths.push(RoutedArrowPath::new(f64_points));
                } else {
                    // Fallback to straight line if routing fails
                    routed_paths.push(RoutedArrowPath::new(vec![start, end]));
                }
            } else {
                // Port not found, push empty path
                routed_paths.push(RoutedArrowPath::new(Vec::new()));
            }
        }

        routed_paths
    }
}

impl std::fmt::Debug for RoutedArrowPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self.path
            .iter()
            .copied()
            .map(|(row, col)| format!("({row}, {col})"))
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "RoutedArrowPath({path})")
    }
}
