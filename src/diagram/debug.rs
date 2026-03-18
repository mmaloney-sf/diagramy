/// Debug output for diagram structures
///
/// Renders the diagram data in a human-readable syntax for debugging purposes.

use std::fmt::Write;

/// Render a diagram in a human-readable debug format
pub fn debug(diagram: &super::Diagram) -> String {
    let mut output = String::new();

    writeln!(output, "diagram {{").unwrap();

    // Write title if present
    if let Some(ref title) = diagram.title {
        writeln!(output, "    title: \"{}\"", title).unwrap();
    }

    // Write color if present
    if let Some(ref color) = diagram.color {
        writeln!(output, "    color: {}", color).unwrap();
    }

    writeln!(output).unwrap();

    // Write all boxes
    for (i, box_item) in diagram.boxes.iter().enumerate() {
        writeln!(output, "    box #{} {{", i).unwrap();
        writeln!(output, "        pos: {:?}", box_item.pos).unwrap();
        writeln!(output, "        size: {:?}", box_item.size).unwrap();

        if let Some(ref title) = box_item.title {
            writeln!(output, "        title: \"{}\"", title).unwrap();
        }

        if let Some(ref color) = box_item.color {
            writeln!(output, "        color: {}", color).unwrap();
        }

        writeln!(output, "        font_scale: {:.4}", box_item.font_scale).unwrap();
        writeln!(output, "        has_children: {}", box_item.has_children).unwrap();

        if let Some(ref border_style) = box_item.border_style {
            writeln!(output, "        border_style: {}", border_style).unwrap();
        }

        writeln!(output, "        horizontal_scaling: {:.4}", box_item.horizontal_scaling).unwrap();
        writeln!(output, "        vertical_scaling: {:.4}", box_item.vertical_scaling).unwrap();
        writeln!(output, "    }}").unwrap();
        writeln!(output).unwrap();
    }

    // Write all ports
    for port in &diagram.ports {
        writeln!(output, "    port {{").unwrap();
        writeln!(output, "        name: {}", port.name).unwrap();
        writeln!(output, "        pos: {:?}", port.pos).unwrap();
        writeln!(output, "    }}").unwrap();
        writeln!(output).unwrap();
    }

    // Write all arrows
    for arrow in &diagram.arrows {
        writeln!(output, "    arrow {{").unwrap();
        writeln!(output, "        from: {}", arrow.from).unwrap();
        writeln!(output, "        to: {}", arrow.to).unwrap();
        writeln!(output, "    }}").unwrap();
        writeln!(output).unwrap();
    }

    // Write routed paths
    for (i, path) in diagram.routed_paths.iter().enumerate() {
        writeln!(output, "    routed_path #{} {{", i).unwrap();
        writeln!(output, "        points: {} points", path.len()).unwrap();
        if !path.is_empty() {
            writeln!(output, "        start: {:?}", path[0]).unwrap();
            writeln!(output, "        end: {:?}", path[path.len() - 1]).unwrap();
        }
        writeln!(output, "    }}").unwrap();
        writeln!(output).unwrap();
    }

    writeln!(output, "}}").unwrap();

    output
}
