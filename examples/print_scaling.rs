use diagramy::grammar;
use diagramy::elaboration;
use diagramy::diagram;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file.dgmy>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let source = std::fs::read_to_string(filename).expect("Failed to read file");

    // Parse
    let doc = grammar::DocumentParser::new()
        .parse(&source, &source)
        .expect("Parse failed");

    // Elaborate
    let elab = elaboration::from_ast(&doc, &source, filename, None)
        .expect("Elaboration failed");

    // Convert to diagram
    let diagram = diagram::from_elaboration(&elab);

    // Print scaling information for each box
    println!("Box Scaling Information:");
    println!("{:-<80}", "");
    for (i, box_item) in diagram.boxes.iter().enumerate() {
        let title = box_item.title.as_deref().unwrap_or("<no title>");
        let (width, height) = box_item.size;
        println!("Box {}: {}", i, title);
        println!("  Size: {}x{} pixels", width, height);
        println!("  Horizontal scaling: {:.3} (ratio to top box)", box_item.horizontal_scaling);
        println!("  Vertical scaling: {:.3} (ratio to top box)", box_item.vertical_scaling);
        println!("  Average scaling: {:.3} (ratio to top box)", box_item.scaling());
        println!("  Font scale: {:.3} (ratio to canvas width)", box_item.font_scale);
        println!();
    }
}

