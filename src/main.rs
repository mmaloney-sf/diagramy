// Include the generated parser module
#[macro_use] extern crate lalrpop_util;

use svg::Document;
use svg::node::element::{Circle, Line, Text, Rectangle};

mod ast;
use ast::{Expr, Opcode};

lalrpop_mod!(pub calculator); // synthesized by LALRPOP

// Evaluate the AST to get the result
fn eval(expr: &Expr) -> i32 {
    match expr {
        Expr::Number(n) => *n,
        Expr::Op(l, op, r) => {
            let left = eval(l);
            let right = eval(r);
            match op {
                Opcode::Add => left + right,
                Opcode::Sub => left - right,
                Opcode::Mul => left * right,
                Opcode::Div => left / right,
            }
        }
    }
}

// Render the AST as an SVG tree diagram
fn render_ast_to_svg(expr: &Expr, filename: &str) {
    const NODE_RADIUS: f32 = 25.0;
    const LEVEL_HEIGHT: f32 = 100.0;
    const MIN_SPACING: f32 = 80.0;

    // Calculate tree dimensions
    fn tree_width(expr: &Expr) -> usize {
        match expr {
            Expr::Number(_) => 1,
            Expr::Op(l, _, r) => tree_width(l) + tree_width(r),
        }
    }

    let width = tree_width(expr).max(3) * MIN_SPACING as usize;

    fn tree_height(expr: &Expr) -> usize {
        match expr {
            Expr::Number(_) => 1,
            Expr::Op(l, _, r) => 1 + tree_height(l).max(tree_height(r)),
        }
    }

    let height = tree_height(expr) * LEVEL_HEIGHT as usize + 100;

    let mut document = Document::new()
        .set("viewBox", (0, 0, width, height))
        .set("width", width)
        .set("height", height);

    // Add background
    let background = Rectangle::new()
        .set("width", "100%")
        .set("height", "100%")
        .set("fill", "#f8f9fa");
    document = document.add(background);

    // Recursive function to draw the tree
    fn draw_node(
        expr: &Expr,
        x: f32,
        y: f32,
        width: f32,
        doc: Document,
    ) -> Document {
        let mut doc = doc;

        match expr {
            Expr::Number(n) => {
                // Draw circle for number
                let circle = Circle::new()
                    .set("cx", x)
                    .set("cy", y)
                    .set("r", NODE_RADIUS)
                    .set("fill", "#4CAF50")
                    .set("stroke", "#2E7D32")
                    .set("stroke-width", 2);
                doc = doc.add(circle);

                // Draw text
                let text = Text::new(n.to_string())
                    .set("x", x)
                    .set("y", y + 5.0)
                    .set("text-anchor", "middle")
                    .set("font-size", 16)
                    .set("font-weight", "bold")
                    .set("fill", "white");
                doc = doc.add(text);
            }
            Expr::Op(left, op, right) => {
                // Calculate positions for children
                let left_x = x - width / 4.0;
                let right_x = x + width / 4.0;
                let child_y = y + LEVEL_HEIGHT;

                // Draw lines to children
                let line_left = Line::new()
                    .set("x1", x)
                    .set("y1", y + NODE_RADIUS)
                    .set("x2", left_x)
                    .set("y2", child_y - NODE_RADIUS)
                    .set("stroke", "#666")
                    .set("stroke-width", 2);
                doc = doc.add(line_left);

                let line_right = Line::new()
                    .set("x1", x)
                    .set("y1", y + NODE_RADIUS)
                    .set("x2", right_x)
                    .set("y2", child_y - NODE_RADIUS)
                    .set("stroke", "#666")
                    .set("stroke-width", 2);
                doc = doc.add(line_right);

                // Draw circle for operator
                let circle = Circle::new()
                    .set("cx", x)
                    .set("cy", y)
                    .set("r", NODE_RADIUS)
                    .set("fill", "#2196F3")
                    .set("stroke", "#1565C0")
                    .set("stroke-width", 2);
                doc = doc.add(circle);

                // Draw operator text
                let op_str = match op {
                    Opcode::Add => "+",
                    Opcode::Sub => "-",
                    Opcode::Mul => "×",
                    Opcode::Div => "÷",
                };
                let text = Text::new(op_str)
                    .set("x", x)
                    .set("y", y + 6.0)
                    .set("text-anchor", "middle")
                    .set("font-size", 20)
                    .set("font-weight", "bold")
                    .set("fill", "white");
                doc = doc.add(text);

                // Recursively draw children
                doc = draw_node(left, left_x, child_y, width / 2.0, doc);
                doc = draw_node(right, right_x, child_y, width / 2.0, doc);
            }
        }

        doc
    }

    document = draw_node(expr, width as f32 / 2.0, 60.0, width as f32, document);

    // Save to file
    svg::save(filename, &document).unwrap();
    println!("Saved AST diagram to: {}", filename);
}

fn main() {
    // Create a parser instance
    let parser = calculator::ExprParser::new();

    // Test some expressions
    let test_cases = vec![
        ("22", "ast_simple.svg"),
        ("22 + 33", "ast_addition.svg"),
        ("22 * 44 + 66", "ast_precedence.svg"),
        ("1 + 2 * 3", "ast_multiply_first.svg"),
        ("(1 + 2) * 3", "ast_parentheses.svg"),
        ("10 / 2 - 3", "ast_division.svg"),
    ];

    println!("Basic LALRPOP Calculator with AST Rendering\n");
    println!("============================================\n");

    for (expr_str, filename) in test_cases {
        match parser.parse(expr_str) {
            Ok(ast) => {
                let result = eval(&ast);
                println!("{} = {}", expr_str, result);
                render_ast_to_svg(&ast, filename);
            }
            Err(e) => println!("Error parsing '{}': {:?}", expr_str, e),
        }
    }

    println!("\nAll AST diagrams have been generated!");
}
