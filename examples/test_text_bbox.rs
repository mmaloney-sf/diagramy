use diagramy::diagram::DiagramBox;

fn main() {
    println!("Text Bounding Box Estimation Examples");
    println!("{:=<60}", "");
    
    // Test 1: Single line text
    let text1 = "Hello World";
    let font_size1 = 18;
    let (width1, height1) = DiagramBox::estimate_text_bbox(text1, font_size1);
    println!("\nTest 1: Single line");
    println!("  Text: \"{}\"", text1);
    println!("  Font size: {}px", font_size1);
    println!("  Estimated bbox: {}px × {}px", width1, height1);
    println!("  Characters: {}", text1.chars().count());
    
    // Test 2: Multi-line text
    let text2 = "Hello\nWorld\nTest";
    let font_size2 = 18;
    let (width2, height2) = DiagramBox::estimate_text_bbox(text2, font_size2);
    println!("\nTest 2: Multi-line text");
    println!("  Text: \"{}\"", text2.replace('\n', "\\n"));
    println!("  Font size: {}px", font_size2);
    println!("  Estimated bbox: {}px × {}px", width2, height2);
    println!("  Lines: {}", text2.split('\n').count());
    
    // Test 3: Different font sizes
    let text3 = "Component";
    for font_size in [12, 18, 24, 36] {
        let (width, height) = DiagramBox::estimate_text_bbox(text3, font_size);
        println!("\nTest 3: Font size {}px", font_size);
        println!("  Text: \"{}\"", text3);
        println!("  Estimated bbox: {}px × {}px", width, height);
    }
    
    // Test 4: Wide vs narrow text
    let text4a = "WWWWWWWWWW";  // Wide characters
    let text4b = "iiiiiiiiii";  // Narrow characters
    let font_size4 = 18;
    let (width4a, height4a) = DiagramBox::estimate_text_bbox(text4a, font_size4);
    let (width4b, height4b) = DiagramBox::estimate_text_bbox(text4b, font_size4);
    println!("\nTest 4: Character width variation (approximation)");
    println!("  Wide text: \"{}\" → {}px × {}px", text4a, width4a, height4a);
    println!("  Narrow text: \"{}\" → {}px × {}px", text4b, width4b, height4b);
    println!("  Note: Both estimate the same width (approximation limitation)");
    
    // Test 5: Empty and whitespace
    let text5a = "";
    let text5b = "   ";
    let font_size5 = 18;
    let (width5a, height5a) = DiagramBox::estimate_text_bbox(text5a, font_size5);
    let (width5b, height5b) = DiagramBox::estimate_text_bbox(text5b, font_size5);
    println!("\nTest 5: Edge cases");
    println!("  Empty string: \"{}\" → {}px × {}px", text5a, width5a, height5a);
    println!("  Whitespace: \"{}\" → {}px × {}px", text5b, width5b, height5b);
    
    // Test 6: Uneven line lengths
    let text6 = "Short\nMedium line\nVery long line here";
    let font_size6 = 18;
    let (width6, height6) = DiagramBox::estimate_text_bbox(text6, font_size6);
    println!("\nTest 6: Uneven line lengths");
    println!("  Text lines:");
    for (i, line) in text6.split('\n').enumerate() {
        println!("    Line {}: \"{}\" ({} chars)", i+1, line, line.chars().count());
    }
    println!("  Font size: {}px", font_size6);
    println!("  Estimated bbox: {}px × {}px", width6, height6);
    println!("  (Width based on longest line: \"Very long line here\")");
}

