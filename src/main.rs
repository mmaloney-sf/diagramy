use std::fs;
use diagramy::{grammar, render_diagram_to_svg};

fn main() {
    // Create build directory if it doesn't exist
    std::fs::create_dir_all("build").expect("Failed to create build directory");

    // Create a parser instance
    let parser = grammar::DocumentParser::new();

    // Read the input file from command line argument
    let input_file = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: diagramy <input.dia>");
        std::process::exit(1);
    });

    // Read the input file
    let input = fs::read_to_string(&input_file)
        .expect(&format!("Failed to read {}", input_file));

    // Generate output filename: extract base name and create .svg in build/
    let output_file = {
        use std::path::Path;
        let path = Path::new(&input_file);
        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("diagram");
        format!("build/{}.svg", stem)
    };

    println!("Diagram Parser\n");
    println!("==============\n");

    match parser.parse(&input) {
        Ok(doc) => {
            println!("Successfully parsed diagram!");
            println!("Debug AST: {:#?}\n", doc);
            render_diagram_to_svg(&doc, &output_file);
        }
        Err(e) => {
            println!("Error parsing diagram: {:?}", e);
        }
    }
}
