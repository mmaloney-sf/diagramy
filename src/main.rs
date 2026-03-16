use std::fs;
use clap::Parser;
use diagramy::grammar;
use lalrpop_util::ParseError;

/// Convert .dia diagram files to SVG
#[derive(Parser, Debug)]
#[command(name = "diagramy")]
#[command(about = "A diagram file to SVG converter", long_about = None)]
struct Args {
    /// Input .dia file to parse
    #[arg(required = true)]
    file: String,

    /// Parse only and print AST (don't render)
    #[arg(long)]
    parse: bool,

    /* TODO: Not yet implemented
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
    */
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

    // Read the input file
    let input = match fs::read_to_string(&args.file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading {}: {}", args.file, e);
            std::process::exit(1);
        }
    };

    // Create a parser instance
    let parser = grammar::DocumentParser::new();

    // Parse the file
    match parser.parse(&input) {
        Ok(doc) => {
            if args.parse {
                println!("{:#?}", doc);
            } else {
                dbg!(&doc);
            }
        }
        Err(e) => {
            print_parse_error(&args.file, &input, &e);
            std::process::exit(1);
        }
    }
}
