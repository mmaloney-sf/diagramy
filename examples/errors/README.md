# Error Examples

This directory contains example `.dia` files that demonstrate various parse and validation errors.
These files are intentionally invalid and are used to test error reporting.

## Parse Error Files

- **unrecognized_token.dia** - Contains an invalid property name that the parser doesn't recognize
- **unrecognized_eof.dia** - File ends unexpectedly before all braces are closed
- **invalid_token.dia** - Contains an invalid character (@) in a numeric position
- **missing_colon.dia** - Missing colon after a property name

## Validation Error Files

- **overlapping_boxes.dia** - Two sibling boxes that overlap (not allowed)
- **child_outside_parent.dia** - Child box extends beyond parent boundaries (not allowed)

## Testing Error Reporting

You can test the parse error reporting by running:

```bash
cargo run examples/errors/unrecognized_token.dia
cargo run examples/errors/unrecognized_eof.dia
cargo run examples/errors/invalid_token.dia
cargo run examples/errors/missing_colon.dia
```

Each command will display detailed parse error information including:
- Error type/name
- Line and column number
- The problematic token (when available)
- Expected tokens (when available)

You can test the validation error reporting by running:

```bash
cargo run examples/errors/overlapping_boxes.dia
cargo run examples/errors/child_outside_parent.dia
```

Each command will display detailed validation error information including:
- Box identifiers
- Box positions and sizes
- Description of the validation failure

