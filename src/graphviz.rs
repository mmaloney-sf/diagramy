use crate::elaboration::{ElaboratedDiagram, BoxDef, Box as ElabBox};
use std::collections::HashMap;

/// Convert an elaborated diagram to GraphViz DOT format
pub fn to_dot(elab_diagram: &ElaboratedDiagram) -> String {
    let mut output = String::new();
    
    // Start digraph
    output.push_str("digraph diagram {\n");
    output.push_str("    rankdir=LR;\n");
    output.push_str("    node [shape=box];\n");
    output.push_str("\n");
    
    // Track node IDs
    let mut node_counter = 0;
    let mut node_map: HashMap<String, String> = HashMap::new();
    
    // Process the top-level box and its children
    // Collect nodes by row for rank grouping (only for non-clustered, non-port nodes)
    let mut nodes_by_row: HashMap<usize, Vec<String>> = HashMap::new();
    let mut clustered_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut port_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();

    process_box_def(
        &elab_diagram.top,
        None,
        &mut output,
        &mut node_counter,
        &mut node_map,
        0,
        &mut nodes_by_row,
        &mut clustered_nodes,
        &mut port_nodes,
    );

    // Add rank constraints to respect grid layout (only for non-clustered, non-port nodes)
    // Note: GraphViz has issues with rank constraints on record-shaped nodes with ports
    output.push_str("\n    // Grid layout constraints\n");
    let mut rows: Vec<_> = nodes_by_row.keys().collect();
    rows.sort();
    for row in rows {
        if let Some(nodes) = nodes_by_row.get(row) {
            // Filter out clustered nodes and nodes with ports
            let eligible: Vec<_> = nodes.iter()
                .filter(|n| !clustered_nodes.contains(*n) && !port_nodes.contains(*n))
                .collect();
            if !eligible.is_empty() {
                output.push_str(&format!("    {{ rank=same; {}; }}\n",
                    eligible.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("; ")));
            }
        }
    }
    
    // Add arrows
    output.push_str("\n    // Arrows\n");
    collect_and_render_arrows(&elab_diagram.top, &node_map, &mut output);
    
    output.push_str("}\n");
    output
}

/// Process a box definition and its children
fn process_box_def(
    box_def: &BoxDef,
    box_id: Option<&str>,
    output: &mut String,
    node_counter: &mut usize,
    node_map: &mut HashMap<String, String>,
    indent_level: usize,
    nodes_by_row: &mut HashMap<usize, Vec<String>>,
    clustered_nodes: &mut std::collections::HashSet<String>,
    port_nodes: &mut std::collections::HashSet<String>,
) {
    let indent = "    ".repeat(indent_level + 1);

    // Create a node ID for this box
    let node_id = if let Some(id) = box_id {
        // Use the box's ID if it has one
        id.to_string()
    } else {
        // Generate a unique ID
        let id = format!("node{}", node_counter);
        *node_counter += 1;
        id
    };

    // Add to node map if this box has an ID
    if let Some(id) = box_id {
        node_map.insert(id.to_string(), node_id.clone());
    }

    // Add ports to node map with proper GraphViz syntax
    // Ports can be referenced by their full qualified name (e.g., "core.clock")
    // We need to map these to GraphViz port syntax (e.g., "core:clock")
    for port in &box_def.ports {
        let port_node_id = format!("{}:{}", node_id, sanitize_port_name(&port.name));

        // Store both the simple port name and the qualified name if box has an ID
        node_map.insert(port.name.clone(), port_node_id.clone());

        // Also store the qualified name (box_id.port_name) if this box has an ID
        if let Some(id) = box_id {
            let qualified_name = format!("{}.{}", id, port.name);
            node_map.insert(qualified_name, port_node_id);
        }
    }

    // If this box has children, create a cluster (subgraph) for better hierarchy visualization
    let has_children = !box_def.boxes.is_empty();
    if has_children && box_id.is_some() {
        // Mark this node and all its children as clustered
        clustered_nodes.insert(node_id.clone());

        output.push_str(&format!("{}subgraph cluster_{} {{\n", indent, node_id));
        output.push_str(&format!("{}    label=\"{}\";\n", indent,
            box_def.title.as_ref().unwrap_or(&node_id)));

        // Add cluster styling
        if let Some(ref color) = box_def.color {
            output.push_str(&format!("{}    bgcolor=\"{}\";\n", indent, color));
        }
        output.push_str(&format!("{}    style=rounded;\n", indent));
    }
    
    // Determine the label for this node
    let label = if let Some(ref title) = box_def.title {
        escape_label(title)
    } else if let Some(id) = box_id {
        escape_label(id)
    } else {
        "Box".to_string()
    };
    
    // Build node attributes
    let mut attrs = vec![format!("label=\"{}\"", label)];
    
    // Add color if specified
    if let Some(ref color) = box_def.color {
        attrs.push(format!("fillcolor=\"{}\"", color));
        attrs.push("style=filled".to_string());
    }
    
    // Add border style
    if let Some(ref border_style) = box_def.border_style {
        match border_style.as_str() {
            "none" => attrs.push("style=invis".to_string()),
            "dotted" => attrs.push("style=dotted".to_string()),
            "dashed" => attrs.push("style=dashed".to_string()),
            _ => {}
        }
    }
    
    // If this box has ports, use record shape
    if !box_def.ports.is_empty() {
        // Mark this node as having ports (to avoid rank constraints)
        port_nodes.insert(node_id.clone());

        attrs.clear();
        let mut record_label = String::new();
        record_label.push('{');

        // Add ports on the left
        for (i, port) in box_def.ports.iter().enumerate() {
            if i > 0 {
                record_label.push('|');
            }
            let port_label = port.label.as_ref().unwrap_or(&port.name);
            record_label.push_str(&format!("<{}> {}", sanitize_port_name(&port.name), escape_label(port_label)));
        }

        // Add the main label
        record_label.push('|');
        record_label.push_str(&label);
        record_label.push('}');

        attrs.push(format!("label=\"{}\"", record_label));
        attrs.push("shape=record".to_string());

        if let Some(ref color) = box_def.color {
            attrs.push(format!("fillcolor=\"{}\"", color));
            attrs.push("style=filled".to_string());
        }
    }
    
    // Write the node (adjust indent if inside a cluster)
    let node_indent = if has_children && box_id.is_some() {
        format!("{}    ", indent)
    } else {
        indent.clone()
    };
    output.push_str(&format!("{}{}[{}];\n", node_indent, node_id, attrs.join(", ")));

    // Process child boxes
    for child_box in &box_def.boxes {
        let child_indent = if has_children && box_id.is_some() {
            indent_level + 1
        } else {
            indent_level
        };
        process_child_box(
            child_box,
            output,
            node_counter,
            node_map,
            child_indent,
            nodes_by_row,
            clustered_nodes,
            port_nodes,
            has_children && box_id.is_some(),
        );
    }

    // Close the cluster if we opened one
    if has_children && box_id.is_some() {
        output.push_str(&format!("{}}}\n", indent));
    }
}

/// Process a child box
fn process_child_box(
    child_box: &ElabBox,
    output: &mut String,
    node_counter: &mut usize,
    node_map: &mut HashMap<String, String>,
    indent_level: usize,
    nodes_by_row: &mut HashMap<usize, Vec<String>>,
    clustered_nodes: &mut std::collections::HashSet<String>,
    port_nodes: &mut std::collections::HashSet<String>,
    parent_is_cluster: bool,
) {
    // Track the node by its row position for rank grouping
    let row = child_box.pos.0;
    let node_id = if let Some(ref id) = child_box.id {
        id.clone()
    } else {
        format!("node{}", node_counter)
    };

    // Only add to nodes_by_row if not in a cluster
    if !parent_is_cluster {
        nodes_by_row.entry(row).or_insert_with(Vec::new).push(node_id.clone());
    } else {
        // Mark as clustered
        clustered_nodes.insert(node_id.clone());
    }

    process_box_def(
        &child_box.def,
        child_box.id.as_deref(),
        output,
        node_counter,
        node_map,
        indent_level,
        nodes_by_row,
        clustered_nodes,
        port_nodes,
    );
}

/// Recursively collect and render arrows from a box and its children
fn collect_and_render_arrows(
    box_def: &BoxDef,
    node_map: &HashMap<String, String>,
    output: &mut String,
) {
    // Render arrows from this box
    for arrow in &box_def.arrows {
        let from_node = node_map.get(&arrow.from).unwrap_or_else(|| {
            // If not found in map, try to convert dot notation to colon notation
            if arrow.from.contains('.') {
                &arrow.from
            } else {
                &arrow.from
            }
        });
        let to_node = node_map.get(&arrow.to).unwrap_or_else(|| {
            // If not found in map, try to convert dot notation to colon notation
            if arrow.to.contains('.') {
                &arrow.to
            } else {
                &arrow.to
            }
        });

        // Convert dot notation to colon notation for GraphViz
        let from_graphviz = from_node.replace('.', ":");
        let to_graphviz = to_node.replace('.', ":");

        output.push_str(&format!("    {} -> {};\n", from_graphviz, to_graphviz));
    }

    // Recursively collect from child boxes
    for child_box in &box_def.boxes {
        collect_and_render_arrows(&child_box.def, node_map, output);
    }
}

/// Escape special characters in labels for GraphViz
fn escape_label(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Sanitize port names for use in GraphViz (remove special characters)
fn sanitize_port_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

/// Write a DOT file to disk
pub fn write_dot_file(filename: &str, dot_content: &str) -> Result<(), String> {
    use std::fs;
    fs::write(filename, dot_content)
        .map_err(|e| format!("Failed to write DOT file: {}", e))
}

