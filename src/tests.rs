use std::fs;
use std::path::Path;

#[test]
fn test_examples() {
    // Get all .dgmy files in the examples directory
    let examples_dir = Path::new("examples");
    
    if !examples_dir.exists() {
        panic!("Examples directory not found");
    }
    
    let mut example_files = Vec::new();
    
    // Read all .dgmy files from the examples directory
    for entry in fs::read_dir(examples_dir).expect("Failed to read examples directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("dgmy") {
            example_files.push(path);
        }
    }
    
    // Sort for consistent test order
    example_files.sort();
    
    if example_files.is_empty() {
        panic!("No example files found in examples directory");
    }
    
    println!("Found {} example files", example_files.len());
    
    // Validate each example file
    for example_path in &example_files {
        let filename = example_path.to_str().unwrap();
        println!("Validating: {}", filename);
        
        // Read the file content
        let content = fs::read_to_string(example_path)
            .expect(&format!("Failed to read file: {}", filename));

        // Parse the file
        let parser = crate::grammar::DocumentParser::new();
        let doc = parser.parse(&content, &content)
            .expect(&format!("Failed to parse {}", filename));

        // Validate the document
        let validation_result = crate::validation::validate(&doc, &content, filename);

        // Assert that validation succeeded
        assert!(
            validation_result.is_ok(),
            "Validation failed for {}: {}",
            filename,
            validation_result.err().unwrap()
        );
    }
    
    println!("All {} examples validated successfully", example_files.len());
}

