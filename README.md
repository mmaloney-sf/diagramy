# Diagramy

A declarative markup language for creating grid-based block diagrams. Diagramy makes it easy to create clean, professional diagrams for system architectures, network topologies, and other structured visualizations.

## Features

- **Grid-based layout system** - Position boxes using simple row/column coordinates
- **Auto-positioning** - Boxes automatically fill the next available grid cell
- **Multi-cell spanning** - Boxes can span multiple rows and columns
- **Nested hierarchies** - Create complex diagrams with boxes inside boxes
- **Reusable components** - Define box templates and reuse them throughout your diagram
- **Customizable styling** - Colors, border styles, and multi-line text
- **Simple syntax** - Clean, readable markup language

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/dgmy`.

## Quick Start

Create a file called `example.dgmy`:

```
diagram {
    width: 400
    color: grey
    text: "My First Diagram"
}

box top {
    grid: 2x2

    box is {
        color: red
        text: "Box A"
    }

    box is {
        color: green
        text: "Box B"
    }

    box is {
        color: blue
        text: "Box C"
    }

    box is {
        color: yellow
        text: "Box D"
    }
}
```

Render it to SVG:

```bash
dgmy example.dgmy
```

This creates `example.svg`.
Open it with:

```bash
dgmy example.dgmy --open
```

## Language Reference

### Diagram Section

Every file starts with a `diagram` section that defines global properties:

```
diagram {
    version: "0.1.0"    // Optional version string
    width: 800          // Diagram width in pixels
    color: grey         // Background color
    text: "Title"       // Diagram title
}
```

### Box Definitions

Boxes are the building blocks of your diagram. There are two types:

**1. Reusable Box Definitions**

Define a box template that can be reused:

```
box MyComponent {
    grid: 2x2
    color: blue

    box at (1, 1) is {
        text: "Part A"
    }

    box at (1, 2) is {
        text: "Part B"
    }
}
```

**2. Box Instances**

Use boxes in your diagram:

```
box top {
    grid: 3x3

    // Reference a defined box
    box at (1, 1) is MyComponent

    // Inline box definition
    box at (1, 2) is {
        color: red
        text: "Inline Box"
    }
}
```

### Grid System

Every box with children must define a grid:

```
box parent {
    grid: 3x4  // 3 rows, 4 columns

    // Child boxes are positioned within this grid
}
```

### Positioning

**Explicit positioning:**
```
box at (2, 3) is {  // Row 2, Column 3 (1-based)
    text: "Fixed Position"
}
```

**Auto-positioning:**
```
box is {  // Automatically placed in next available cell
    text: "Auto Position"
}
```

### Dimensions

Boxes can span multiple grid cells:

```
box at (1, 1) dim 2x3 is {  // 2 rows, 3 columns
    text: "Large Box"
}
```

### Properties

**Box Properties:**
- `grid: HxW` - Define grid size for child boxes
- `color: <color>` - Background color
- `text: "..."` - Text content (can be multi-line)
- `borderStyle: <style>` - Border style (solid, dotted, dashed, none)

**Available Colors:**
red, green, blue, yellow, purple, cyan, orange, pink, white, grey

**Multi-line Text:**
```
text: "Line 1"
      "Line 2"
      "Line 3"
```

## Examples

The `examples/` directory contains 12 comprehensive examples:

1. **[01_simple_grid.dgmy](examples/01_simple_grid.dgmy)** - Basic 2x2 grid layout ([rendered](assets/images/01_simple_grid.svg))
2. **[02_auto_positioning.dgmy](examples/02_auto_positioning.dgmy)** - Automatic box positioning ([rendered](assets/images/02_auto_positioning.svg))
3. **[03_dimensions.dgmy](examples/03_dimensions.dgmy)** - Boxes spanning multiple cells ([rendered](assets/images/03_dimensions.svg))
4. **[04_nested_boxes.dgmy](examples/04_nested_boxes.dgmy)** - Hierarchical box structures ([rendered](assets/images/04_nested_boxes.svg))
5. **[05_reusable_definitions.dgmy](examples/05_reusable_definitions.dgmy)** - Component reuse ([rendered](assets/images/05_reusable_definitions.svg))
6. **[06_multiline_text.dgmy](examples/06_multiline_text.dgmy)** - Multi-line text labels ([rendered](assets/images/06_multiline_text.svg))
7. **[07_border_styles.dgmy](examples/07_border_styles.dgmy)** - Different border styles ([rendered](assets/images/07_border_styles.svg))
8. **[08_color_palette.dgmy](examples/08_color_palette.dgmy)** - All available colors ([rendered](assets/images/08_color_palette.svg))
9. **[09_system_architecture.dgmy](examples/09_system_architecture.dgmy)** - Computer system diagram ([rendered](assets/images/09_system_architecture.svg))
10. **[10_network_topology.dgmy](examples/10_network_topology.dgmy)** - Network layout ([rendered](assets/images/10_network_topology.svg))
11. **[11_mixed_features.dgmy](examples/11_mixed_features.dgmy)** - Combining multiple features ([rendered](assets/images/11_mixed_features.svg))
12. **[12_microservice_architecture.dgmy](examples/12_microservice_architecture.dgmy)** - Complex microservice system ([rendered](assets/images/12_microservice_architecture.svg))

Render all examples:

```bash
for f in examples/*.dgmy; do dgmy "$f"; done
```

## Command-Line Options

```
dgmy <file>              # Render to SVG (default)
dgmy <file> --open       # Render and open in default viewer
dgmy <file> --parse      # Parse and print AST (debug)
dgmy <file> --validate   # Validate and print AST (debug)
```
