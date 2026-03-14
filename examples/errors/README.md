# Error Examples

This directory contains example `.dia` files that demonstrate various parse errors.
These files are intentionally invalid and are used to test error reporting.

## Files

- **unrecognized_token.dia** - Contains an invalid property name that the parser doesn't recognize
- **unrecognized_eof.dia** - File ends unexpectedly before all braces are closed
- **invalid_token.dia** - Contains an invalid character (@) in a numeric position
- **missing_colon.dia** - Missing colon after a property name

## Testing Error Reporting

You can test the error reporting by running:

```bash
cargo run examples/errors/unrecognized_token.dia
cargo run examples/errors/unrecognized_eof.dia
cargo run examples/errors/invalid_token.dia
cargo run examples/errors/missing_colon.dia
```

Each command will display detailed error information including:
- Error type/name
- Line and column number
- The problematic token (when available)
- Expected tokens (when available)

