# Diagramy

A declarative diagram language and rendering engine for creating technical block diagrams with precise layout control. Diagramy separates diagram structure from layout, enabling maintainable, version-controllable diagram specifications.

## Overview

Diagramy is a domain-specific language (DSL) for technical diagrams commonly found in hardware and software architecture documentation. It provides:

- **Declarative syntax** for defining hierarchical box structures
- **Separate layout specification** decoupled from diagram structure
- **Port-based connectivity** with automatic arrow routing
- **Scalable rendering** with configurable font sizes and visual elements
- **Color inheritance** through the diagram hierarchy
- **SVG output** for high-quality, scalable graphics

![Reference](https://raw.githubusercontent.com/mmaloney-sf/diagramy/refs/heads/main/images/reference.svg)

## Core Concepts

### Diagram Structure

A `.dia` file consists of three main sections:

1. **Version declaration** - Specifies the file format version
2. **Diagram block** - Defines the logical structure (boxes, ports, arrows)
3. **Layout block** - Specifies the visual positioning and sizing

This separation allows you to modify the visual layout without changing the logical structure, and vice versa.

### Boxes

Boxes are the primary visual elements. They can:
- Contain other boxes (hierarchical nesting)
- Have properties: `title`, `color`, `vertical` (for vertical text), `stacked` (for 3D effect)
- Inherit colors from parent boxes or the diagram-level color
- Be positioned and sized independently in the layout section

### Ports

Ports are connection points on boxes. They can be:
- **Positioned absolutely** using `pos: (x, y)` coordinates
- **Interpolated on box sides** using `interp: N%` with `side: left|right|top|bottom`
- **Styled** with `style: tieoff` to render as circle-with-X markers
- Connected via arrows for signal flow visualization

### Arrows

Arrows connect ports using orthogonal (Manhattan-style) routing:
- Automatically route horizontally, then vertically, then horizontally
- Render with arrowheads at the destination
- Shorten automatically when pointing to tieoff-style ports
- Extend fully to non-tieoff ports for maximum visibility

### Layout

The layout section maps diagram elements to screen coordinates:
- `size: (width, height)` - Canvas dimensions
- `scale: N%` - Rendering scale factor
- `fontsize: N` - Font size for all text elements
- Per-element `pos` and `size` specifications

Coordinates use screen space (top-left origin, Y increases downward).

## Usage

### Basic Command

```bash
diagramy examples/example.dia
```

This generates `build/example.svg` by default.

### Command-Line Options

```
Usage: diagramy [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Input .dia file

Options:
      --output <OUTPUT>          Output SVG file [default: build/<input>.svg]
      --scale <SCALE>            Scale factor (0.0-1.0) [overrides layout scale]
      --no-transparent           Use white background instead of transparent
      --background <BACKGROUND>  Background color [overrides diagram color]
      --font-size <FONT_SIZE>    Font size for text labels [default: 18]
  -h, --help                     Print help
```

## Example

### Simple Diagram

Create a file `example.dia`:

```dia
version = 0.1.0

diagram {
    color: grey

    box system {
        title: "System"
        color: white

        box processor {
            title: "Processor"
            color: blue
        }

        box memory {
            title: "Memory"
            color: green
        }
    }

    port clockSource {
        title: "Clock"
        style: tieoff
    }

    port clockIn {
        side: left
    }

    arrow {
        from: clockSource
        to: clockIn
    }
}

layout {
    size: (800, 600)
    fontsize: 24

    system {
        pos: (200, 150)
        size: (400, 300)
    }

    processor {
        pos: (50, 50)
        size: (150, 100)
    }

    memory {
        pos: (50, 180)
        size: (150, 100)
    }

    clockSource {
        pos: (50, 250)
    }

    clockIn {
        interp: 50%
    }
}
```

### Render the Diagram

```bash
diagramy examples/example.dia
```

This generates `build/example.svg`.

### Output

![Example Diagram](https://raw.githubusercontent.com/mmaloney-sf/diagramy/refs/heads/main/images/example.svg)

The rendered diagram shows:
- A grey canvas background (from `color: grey` at diagram level)
- A white "System" box containing two nested boxes
- A blue "Processor" box and green "Memory" box
- A clock source port with tieoff style (circle-with-X)
- An arrow with orthogonal routing from the clock source to the system

## Language Reference

### Diagram Properties

```dia
diagram {
    color: <color_name>  // Canvas background and default box color
    // ... boxes, ports, arrows
}
```

### Box Properties

```dia
box <identifier> {
    title: "<text>"      // Display text (required for visibility)
    color: <color_name>  // Box fill color (inherits from parent if omitted)
    vertical             // Render title text vertically
    stacked: <N>         // Create 3D stacked effect with N background boxes
    // ... nested boxes and ports
}
```

### Port Properties

```dia
port <identifier> {
    title: "<text>"           // Label text
    side: left|right|top|bottom  // For interpolated ports
    style: tieoff             // Render as circle-with-X marker
}
```

### Arrow Syntax

```dia
arrow {
    from: <port_identifier>
    to: <port_identifier>
}
```

### Layout Properties

```dia
layout {
    size: (width, height)     // Canvas dimensions in pixels
    scale: <N>%               // Rendering scale (e.g., 50%)
    fontsize: <N>             // Font size in pixels

    <identifier> {
        pos: (x, y)           // Absolute position (top-left origin)
        size: (width, height) // Element dimensions
        interp: <N>%          // Port position along parent box side
    }
}
```

### Supported Colors

Greyscale: `gray`, `grey`, `black`, `white`, `navy`

Chromatic (desaturated): `red`, `blue`, `green`, `yellow`, `orange`, `purple`, `pink`, `cyan`, `magenta`, `lime`, `teal`, `indigo`, `brown`, `maroon`, `olive`

All chromatic colors are rendered with high desaturation and brightness for a professional, subtle appearance.

## Advanced Features

### Color Inheritance

Colors cascade through the hierarchy:
1. Explicit box `color` property
2. Parent box color
3. Diagram-level `color` property
4. Default (gray)

This allows setting a base color scheme at the diagram level and overriding selectively.

### Port Positioning

**Absolute positioning** (for external ports):
```dia
port myPort { }

layout {
    myPort {
        pos: (100, 200)
    }
}
```

**Interpolated positioning** (for ports on box edges):
```dia
box myBox {
    port sidePort {
        side: right
    }
}

layout {
    sidePort {
        interp: 50%  // 50% down the right side
    }
}
```

### Stacked Boxes

Create a 3D effect with background boxes:
```dia
box myBox {
    stacked: 3  // Three background boxes
}
```

Each background box is offset by 12 pixels diagonally.

### Orthogonal Arrow Routing

Arrows automatically route using Manhattan-style paths:
1. Horizontal segment from source
2. Vertical segment at midpoint
3. Horizontal segment to destination

Arrows automatically shorten when pointing to tieoff-style ports to avoid overlapping the circle marker.

## Architecture

Diagramy uses a three-stage pipeline:

1. **Parsing** - LALRPOP-generated parser converts `.dia` text to AST
2. **Validation** - Validates colors, layout references, and structural constraints
3. **Rendering** - Generates SVG with scaled visual elements

The parser is generated from a formal grammar (`src/grammar.lalrpop`), ensuring consistent parsing and enabling future language extensions.
