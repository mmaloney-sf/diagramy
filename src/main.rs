use std::fs;
use std::path::Path;
use clap::Parser;
use diagramy::{grammar, render_diagram_to_svg};
use lalrpop_util::ParseError;

/// Convert .dia diagram files to SVG
#[derive(Parser, Debug)]
#[command(name = "diagramy")]
#[command(about = "A diagram file to SVG converter", long_about = None)]
struct Args {
    /// Input .dia files to convert
    #[arg(required = true)]
    files: Vec<String>,

    /// Scale factor for output (default: from layout or 1.0)
    #[arg(short, long)]
    scale: Option<f64>,

    /// Use white background instead of transparent
    #[arg(long)]
    no_transparent: bool,

    /// Background color (e.g., red, blue, #FFFFFF). If not set, uses transparent or white (with --no-transparent)
    #[arg(long)]
    background: Option<String>,

    /// Font size for text labels (default: 18)
    #[arg(long, default_value = "18")]
    font_size: i32,
}

// Helper function to convert byte offset to line and column
fn get_line_col(input: &str, location: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;

    for (i, ch) in input.chars().enumerate() {
        if i >= location {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

// Print detailed parse error information
fn print_parse_error<T, E>(filename: &str, input: &str, error: &ParseError<usize, T, E>)
where
    T: std::fmt::Display,
    E: std::fmt::Display,
{
    match error {
        ParseError::InvalidToken { location } => {
            let (line, col) = get_line_col(input, *location);
            eprintln!("Parse error in {}: InvalidToken", filename);
            eprintln!("  Location: line {}, column {}", line, col);
        }
        ParseError::UnrecognizedEof { location, expected } => {
            let (line, col) = get_line_col(input, *location);
            eprintln!("Parse error in {}: UnrecognizedEof", filename);
            eprintln!("  Location: line {}, column {}", line, col);
            if !expected.is_empty() {
                eprintln!("  Expected one of: {}", expected.join(", "));
            }
        }
        ParseError::UnrecognizedToken { token: (start, tok, _end), expected } => {
            let (line, col) = get_line_col(input, *start);
            eprintln!("Parse error in {}: UnrecognizedToken", filename);
            eprintln!("  Location: line {}, column {}", line, col);
            eprintln!("  Token: {}", tok);
            if !expected.is_empty() {
                eprintln!("  Expected one of: {}", expected.join(", "));
            }
        }
        ParseError::ExtraToken { token: (start, tok, _end) } => {
            let (line, col) = get_line_col(input, *start);
            eprintln!("Parse error in {}: ExtraToken", filename);
            eprintln!("  Location: line {}, column {}", line, col);
            eprintln!("  Token: {}", tok);
        }
        ParseError::User { error } => {
            eprintln!("Parse error in {}: {}", filename, error);
        }
    }
}

fn main() {
    let args = Args::parse();

    // Create build directory if it doesn't exist
    std::fs::create_dir_all("build").expect("Failed to create build directory");

    // Create a parser instance
    let parser = grammar::DocumentParser::new();

    // Process each input file
    for input_file in &args.files {
        println!("Processing: {}", input_file);

        // Read the input file
        let input = match fs::read_to_string(&input_file) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading {}: {}", input_file, e);
                continue;
            }
        };

        // Generate output filename: extract base name and create .svg in build/
        let output_file = {
            let path = Path::new(&input_file);
            let stem = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("diagram");
            format!("build/{}.svg", stem)
        };

        // Parse and render
        match parser.parse(&input) {
            Ok(doc) => {
                println!("Successfully parsed diagram!");

                // Use CLI scale if provided, otherwise use layout scale, otherwise default to 1.0
                let scale_factor = args.scale
                    .or(doc.layout.scale)
                    .unwrap_or(1.0);

                // Use layout fontsize if provided, otherwise use CLI font_size
                let font_size = doc.layout.fontsize.unwrap_or(args.font_size);

                render_diagram_to_svg(&doc, &output_file, scale_factor, !args.no_transparent, args.background.as_deref(), font_size);
            }
            Err(e) => {
                print_parse_error(&input_file, &input, &e);
            }
        }

        println!();
    }
}
