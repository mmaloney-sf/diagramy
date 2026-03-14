use std::fs;
use std::path::Path;
use clap::Parser;
use diagramy::{grammar, render_diagram_to_svg};

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

                render_diagram_to_svg(&doc, &output_file, scale_factor, !args.no_transparent);
            }
            Err(e) => {
                eprintln!("Error parsing {}: {:?}", input_file, e);
            }
        }

        println!();
    }
}
