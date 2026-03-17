use std::fs;
use std::path::Path;
use std::process::Command;
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

    /// Convert AST to diagram and print (for testing)
    #[arg(long)]
    convert: bool,

    /// Render the diagram to an SVG file
    #[arg(long)]
    render: bool,

    /// Open the rendered SVG file after creation (requires --render)
    #[arg(long)]
    open: bool,

    /// Output SVG filename (default: input filename with .svg extension)
    #[arg(short, long)]
    output: Option<String>,

    /// Font size for text labels (default: 18)
    #[arg(long, default_value = "18")]
    font_size: usize,
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

/// Find the appropriate command to open files on this system
fn find_open_command() -> Option<&'static str> {
    // Check for 'open' (macOS)
    if Command::new("which").arg("open").output().ok()?.status.success() {
        return Some("open");
    }

    // Check for 'xdg-open' (Linux)
    if Command::new("which").arg("xdg-open").output().ok()?.status.success() {
        return Some("xdg-open");
    }

    None
}

/// Open a file with the system's default application
fn open_file(path: &str) -> Result<(), String> {
    let cmd = find_open_command()
        .ok_or_else(|| "Could not find 'open' or 'xdg-open' command on system".to_string())?;

    Command::new(cmd)
        .arg(path)
        .spawn()
        .map_err(|e| format!("Failed to open file: {}", e))?;

    Ok(())
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

    // Validate arguments
    if args.open && !args.render {
        eprintln!("Error: --open requires --render");
        std::process::exit(1);
    }

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
            } else if args.convert {
                // Test the conversion function
                match diagramy::elaboration::from_ast(&doc, &input, &args.file) {
                    Ok(diagram) => {
                        println!("Converted diagram:");
                        println!("  Color: {}", diagram.color);
                        println!("  Size: {:?}", diagram.size);
                        println!("  Top box grid: {:?}", diagram.top.grid);
                        println!("  Top box title: {:?}", diagram.top.title);
                        println!("  Top box has {} child boxes", diagram.top.boxes.len());
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            } else if args.render {
                // Convert AST to elaboration diagram
                let elab_diagram = match diagramy::elaboration::from_ast(&doc, &input, &args.file) {
                    Ok(diagram) => diagram,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                };

                // Convert elaboration diagram to renderable diagram
                let diagram = diagramy::diagram::from_elaboration(&elab_diagram);
                dbg!(&diagram);

                // Determine output filename
                let output_file = args.output.unwrap_or_else(|| {
                    let input_path = Path::new(&args.file);
                    let stem = input_path.file_stem().unwrap().to_str().unwrap();
                    format!("{}.svg", stem)
                });

                // Render to SVG
                let (width, height) = elab_diagram.size;
                match diagram.render_to_svg(&output_file, width, height, args.font_size) {
                    Ok(_) => {
                        println!("Rendered diagram to: {}", output_file);

                        // Open the file if requested
                        if args.open {
                            match open_file(&output_file) {
                                Ok(_) => println!("Opened {}", output_file),
                                Err(e) => eprintln!("Warning: {}", e),
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error rendering diagram: {}", e);
                        std::process::exit(1);
                    }
                }
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
