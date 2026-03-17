use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "diagramy-lsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "diagramy LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text;
        self.documents.write().await.insert(uri.clone(), text.clone());

        // Run validation and publish diagnostics
        self.validate_and_publish_diagnostics(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().next() {
            let text = change.text;
            self.documents.write().await.insert(uri.clone(), text.clone());

            // Run validation and publish diagnostics
            self.validate_and_publish_diagnostics(uri, text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.write().await.remove(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        self.client
            .log_message(
                MessageType::INFO,
                format!("Hover request at {}:{}", position.line, position.character),
            )
            .await;

        // Get the document content
        let documents = self.documents.read().await;
        let text = match documents.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Parse the document
        let parser = diagramy::grammar::DocumentParser::new();
        let doc = match parser.parse(text, text) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };

        // Convert LSP position (0-based) to our position (1-based)
        let target_line = (position.line + 1) as usize;
        let target_col = (position.character + 1) as usize;

        // Try to find what element is being hovered over
        if let Some(hover_text) = get_hover_info(&doc, text, target_line, target_col) {
            return Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String(hover_text)),
                range: None,
            }));
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "Go to definition request at {}:{}",
                    position.line, position.character
                ),
            )
            .await;

        // Get the document content
        let documents = self.documents.read().await;
        let text = match documents.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Parse the document
        let parser = diagramy::grammar::DocumentParser::new();
        let doc = match parser.parse(text, text) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };

        // Convert LSP position (0-based) to our position (1-based)
        let target_line = (position.line + 1) as usize;
        let target_col = (position.character + 1) as usize;

        // Find if we're on a BoxInst::Reference identifier
        if let Some(def_name) = find_box_reference_at_position(&doc, text, target_line, target_col) {
            // Find the corresponding BoxDef
            if let Some(box_def) = doc.box_defs.iter().find(|bd| bd.name == def_name) {
                // Use name_location to jump to the identifier, not the "box" keyword
                let name_span = diagramy::ast::Span::from_offsets(text, box_def.name_location.0, box_def.name_location.1);
                let start = name_span.start();

                // Convert back to LSP position (0-based)
                let location = Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position {
                            line: (start.line() - 1) as u32,
                            character: (start.col() - 1) as u32,
                        },
                        end: Position {
                            line: (start.line() - 1) as u32,
                            character: (start.col() - 1) as u32,
                        },
                    },
                };

                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            }
        }

        // Also check if we're on a "top:" property identifier in the diagram section
        if let Some(def_name) = find_top_prop_at_position(&doc, text, target_line, target_col) {
            // Find the corresponding BoxDef
            if let Some(box_def) = doc.box_defs.iter().find(|bd| bd.name == def_name) {
                // Use name_location to jump to the identifier, not the "box" keyword
                let name_span = diagramy::ast::Span::from_offsets(text, box_def.name_location.0, box_def.name_location.1);
                let start = name_span.start();

                // Convert back to LSP position (0-based)
                let location = Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position {
                            line: (start.line() - 1) as u32,
                            character: (start.col() - 1) as u32,
                        },
                        end: Position {
                            line: (start.line() - 1) as u32,
                            character: (start.col() - 1) as u32,
                        },
                    },
                };

                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            }
        }

        Ok(None)
    }
}

impl Backend {
    /// Validate the document and publish diagnostics
    async fn validate_and_publish_diagnostics(&self, uri: Url, text: String) {
        let mut diagnostics = Vec::new();

        // Parse the document
        let parser = diagramy::grammar::DocumentParser::new();
        let doc = match parser.parse(&text, &text) {
            Ok(d) => d,
            Err(e) => {
                // Parse error - convert to diagnostic with proper line/column
                if let Some(diagnostic) = parse_error_to_diagnostic(&e, &text) {
                    diagnostics.push(diagnostic);
                } else {
                    // Fallback: report at position 0:0
                    let error_msg = format!("{}", e);
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: 0 },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("diagramy".to_string()),
                        message: error_msg,
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }

                self.client.publish_diagnostics(uri, diagnostics, None).await;
                return;
            }
        };

        // Run validation
        let filename = uri.path();
        if let Err(e) = diagramy::validation::validate(&doc, &text, filename) {
            // Validation error - parse the error message to extract position
            // Error format: "filename:line:col: message"
            if let Some(diagnostic) = parse_validation_error(&e) {
                diagnostics.push(diagnostic);
            } else {
                // Fallback: report at position 0:0
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line: 0, character: 0 },
                        end: Position { line: 0, character: 0 },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("diagramy".to_string()),
                    message: e,
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }

        // Publish diagnostics
        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }
}

/// Convert byte offset to line and column (1-based)
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

/// Convert a parse error to a diagnostic with proper line/column numbers
fn parse_error_to_diagnostic(
    error: &lalrpop_util::ParseError<usize, lalrpop_util::lexer::Token, &str>,
    text: &str,
) -> Option<Diagnostic> {
    use lalrpop_util::ParseError;

    match error {
        ParseError::InvalidToken { location } => {
            let (line, col) = get_line_col(text, *location);

            // Check if the invalid token is a semicolon
            if let Some(line_text) = text.lines().nth(line - 1) {
                if let Some(ch) = line_text.chars().nth(col - 1) {
                    if ch == ';' {
                        return Some(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: (line - 1) as u32,
                                    character: (col - 1) as u32,
                                },
                                end: Position {
                                    line: (line - 1) as u32,
                                    character: col as u32,
                                },
                            },
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: None,
                            code_description: None,
                            source: Some("diagramy".to_string()),
                            message: "Remove the semicolon".to_string(),
                            related_information: None,
                            tags: None,
                            data: None,
                        });
                    }
                }
            }

            Some(Diagnostic {
                range: Range {
                    start: Position {
                        line: (line - 1) as u32,
                        character: (col - 1) as u32,
                    },
                    end: Position {
                        line: (line - 1) as u32,
                        character: (col + 5) as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("diagramy".to_string()),
                message: "Invalid token".to_string(),
                related_information: None,
                tags: None,
                data: None,
            })
        }
        ParseError::UnrecognizedEof { location, expected } => {
            let (line, col) = get_line_col(text, *location);
            let message = if expected.is_empty() {
                "Unexpected end of file".to_string()
            } else {
                format!("Unexpected end of file. Expected one of: {}", expected.join(", "))
            };

            Some(Diagnostic {
                range: Range {
                    start: Position {
                        line: (line - 1) as u32,
                        character: (col - 1) as u32,
                    },
                    end: Position {
                        line: (line - 1) as u32,
                        character: col as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("diagramy".to_string()),
                message,
                related_information: None,
                tags: None,
                data: None,
            })
        }
        ParseError::UnrecognizedToken { token: (start, tok, _end), expected } => {
            let (line, col) = get_line_col(text, *start);
            let token_str = tok.to_string();

            // Check if the token is a semicolon
            if token_str == ";" {
                return Some(Diagnostic {
                    range: Range {
                        start: Position {
                            line: (line - 1) as u32,
                            character: (col - 1) as u32,
                        },
                        end: Position {
                            line: (line - 1) as u32,
                            character: col as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("diagramy".to_string()),
                    message: "Remove the semicolon".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }

            let message = if expected.is_empty() {
                format!("Unrecognized token `{}`", token_str)
            } else {
                format!("Unrecognized token `{}`. Expected one of: {}", token_str, expected.join(", "))
            };

            Some(Diagnostic {
                range: Range {
                    start: Position {
                        line: (line - 1) as u32,
                        character: (col - 1) as u32,
                    },
                    end: Position {
                        line: (line - 1) as u32,
                        character: (col + token_str.len() - 1) as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("diagramy".to_string()),
                message,
                related_information: None,
                tags: None,
                data: None,
            })
        }
        ParseError::ExtraToken { token: (start, tok, _end) } => {
            let (line, col) = get_line_col(text, *start);
            let token_str = tok.to_string();

            // Check if the token is a semicolon
            if token_str == ";" {
                return Some(Diagnostic {
                    range: Range {
                        start: Position {
                            line: (line - 1) as u32,
                            character: (col - 1) as u32,
                        },
                        end: Position {
                            line: (line - 1) as u32,
                            character: col as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("diagramy".to_string()),
                    message: "Remove the semicolon".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }

            Some(Diagnostic {
                range: Range {
                    start: Position {
                        line: (line - 1) as u32,
                        character: (col - 1) as u32,
                    },
                    end: Position {
                        line: (line - 1) as u32,
                        character: (col + token_str.len() - 1) as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("diagramy".to_string()),
                message: format!("Extra token `{}`", token_str),
                related_information: None,
                tags: None,
                data: None,
            })
        }
        ParseError::User { error } => {
            Some(Diagnostic {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("diagramy".to_string()),
                message: error.to_string(),
                related_information: None,
                tags: None,
                data: None,
            })
        }
    }
}

/// Parse a validation error message to extract position and create a diagnostic
fn parse_validation_error(error: &str) -> Option<Diagnostic> {
    // Error format: "filename:line:col: message"
    let parts: Vec<&str> = error.splitn(2, ": ").collect();
    if parts.len() != 2 {
        return None;
    }

    let location_parts: Vec<&str> = parts[0].split(':').collect();
    if location_parts.len() < 3 {
        return None;
    }

    // Extract line and column (1-based in error, need 0-based for LSP)
    let line = location_parts[location_parts.len() - 2].parse::<u32>().ok()?;
    let col = location_parts[location_parts.len() - 1].parse::<u32>().ok()?;

    Some(Diagnostic {
        range: Range {
            start: Position {
                line: line.saturating_sub(1),
                character: col.saturating_sub(1),
            },
            end: Position {
                line: line.saturating_sub(1),
                character: col.saturating_sub(1) + 10, // Highlight ~10 characters
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("diagramy".to_string()),
        message: parts[1].to_string(),
        related_information: None,
        tags: None,
        data: None,
    })
}

/// Find if the position is on a BoxInst::Reference identifier and return the def_name
fn find_box_reference_at_position(
    doc: &diagramy::ast::Document,
    text: &str,
    line: usize,
    col: usize,
) -> Option<String> {
    // Search through all box definitions
    for box_def in &doc.box_defs {
        if let Some(name) = search_box_body_for_reference(&box_def.body, text, line, col) {
            return Some(name);
        }
    }
    None
}

/// Recursively search a BoxBody for a BoxInst::Reference at the given position
fn search_box_body_for_reference(
    body: &diagramy::ast::BoxBody,
    text: &str,
    line: usize,
    col: usize,
) -> Option<String> {
    use diagramy::ast::{BoxItem, BoxInst};

    for item in &body.items {
        if let BoxItem::BoxInst(inst) = item {
            match inst {
                BoxInst::Reference { def_name, location, .. } => {
                    // Use the location field which contains byte offsets of just the identifier
                    let ident_span = diagramy::ast::Span::from_offsets(text, location.0, location.1);
                    let start = ident_span.start();
                    let end = ident_span.end();

                    if is_position_in_span(line, col, start.line(), start.col(), end.line(), end.col()) {
                        return Some(def_name.clone());
                    }
                }
                BoxInst::WithBody { body, .. } => {
                    // Recursively search nested bodies
                    if let Some(name) = search_box_body_for_reference(body, text, line, col) {
                        return Some(name);
                    }
                }
            }
        }
    }
    None
}

/// Find if the position is on a "top:" property value in the diagram section
fn find_top_prop_at_position(
    doc: &diagramy::ast::Document,
    text: &str,
    line: usize,
    col: usize,
) -> Option<String> {
    use diagramy::ast::Prop;

    for prop in &doc.diagram.props {
        if let Prop::PropIdent { key, value, value_location, .. } = prop {
            if key == "top" {
                // Check if the position is within the value identifier
                let value_span = diagramy::ast::Span::from_offsets(text, value_location.0, value_location.1);
                let start = value_span.start();
                let end = value_span.end();

                if is_position_in_span(line, col, start.line(), start.col(), end.line(), end.col()) {
                    return Some(value.clone());
                }
            }
        }
    }
    None
}

/// Check if a position is within a span
fn is_position_in_span(
    line: usize,
    col: usize,
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
) -> bool {
    if line < start_line || line > end_line {
        return false;
    }
    if line == start_line && col < start_col {
        return false;
    }
    if line == end_line && col > end_col {
        return false;
    }
    true
}

/// Check if hovering over a box definition reference and provide info about it
fn check_box_reference_hover(
    doc: &diagramy::ast::Document,
    text: &str,
    line: usize,
    col: usize,
) -> Option<String> {
    use diagramy::ast::Prop;

    // Check if hovering over a box reference in a BoxInst::Reference
    for box_def in &doc.box_defs {
        if let Some(hover) = check_box_body_for_reference_hover(&box_def.body, doc, text, line, col) {
            return Some(hover);
        }
    }

    // Check if hovering over the "top" property value
    for prop in &doc.diagram.props {
        if let Prop::PropIdent { key, value, value_location, .. } = prop {
            if key == "top" {
                let value_span = diagramy::ast::Span::from_offsets(text, value_location.0, value_location.1);
                let start = value_span.start();
                let end = value_span.end();

                if is_position_in_span(line, col, start.line(), start.col(), end.line(), end.col()) {
                    // Find the box definition
                    if let Some(box_def) = doc.box_defs.iter().find(|bd| bd.name == *value) {
                        return Some(format_box_def_info(box_def));
                    }
                }
            }
        }
    }

    None
}

/// Recursively check box body for box reference hover
fn check_box_body_for_reference_hover(
    body: &diagramy::ast::BoxBody,
    doc: &diagramy::ast::Document,
    text: &str,
    line: usize,
    col: usize,
) -> Option<String> {
    use diagramy::ast::{BoxItem, BoxInst};

    for item in &body.items {
        if let BoxItem::BoxInst(inst) = item {
            match inst {
                BoxInst::Reference { def_name, location, .. } => {
                    let ident_span = diagramy::ast::Span::from_offsets(text, location.0, location.1);
                    let start = ident_span.start();
                    let end = ident_span.end();

                    if is_position_in_span(line, col, start.line(), start.col(), end.line(), end.col()) {
                        // Find the box definition
                        if let Some(box_def) = doc.box_defs.iter().find(|bd| bd.name == *def_name) {
                            return Some(format_box_def_info(box_def));
                        }
                    }
                }
                BoxInst::WithBody { body, .. } => {
                    if let Some(hover) = check_box_body_for_reference_hover(body, doc, text, line, col) {
                        return Some(hover);
                    }
                }
            }
        }
    }

    None
}

/// Format box definition information for hover display
fn format_box_def_info(box_def: &diagramy::ast::BoxDef) -> String {
    use diagramy::ast::{BoxItem, Prop};

    let name = &box_def.name;

    // Extract grid information
    let mut grid = "(default: 1x1)".to_string();
    let mut text_content: Option<String> = None;
    let mut child_box_count = 0;

    for item in &box_def.body.items {
        match item {
            BoxItem::Prop(prop) => {
                if let Prop::PropDim { key, value, .. } = prop {
                    if key == "grid" {
                        grid = format!("{}x{}", value.height, value.width);
                    }
                } else if let Prop::PropString { key, value, .. } = prop {
                    if key == "text" {
                        text_content = Some(value.join("\n"));
                    }
                }
            }
            BoxItem::BoxInst(_) => {
                child_box_count += 1;
            }
            _ => {}
        }
    }

    // Format the output
    let mut result = format!("```dgmy\n{name}\n```");
    result.push_str(&format!("\n- grid:     {grid}"));

    if child_box_count > 0 {
        result.push_str(&format!("\n- children: {child_box_count}"));
    } else if let Some(text) = text_content {
        // Split multi-line text and align each line
        let lines: Vec<&str> = text.split('\n').collect();
        if lines.len() == 1 {
            result.push_str(&format!("\n- text:     {text}"));
        } else {
            result.push_str("\n- text:     ");
            for (i, line) in lines.iter().enumerate() {
                if i > 0 {
                    result.push_str("\n            "); // Align with first line (12 spaces)
                }
                result.push_str(line);
            }
        }
    }

    result
}

/// Get hover information for the element at the given position
fn get_hover_info(
    doc: &diagramy::ast::Document,
    text: &str,
    line: usize,
    col: usize,
) -> Option<String> {

    // Check if hovering over a property key
    if let Some(hover) = check_property_hover(&doc.diagram.props, text, line, col) {
        return Some(hover);
    }

    // Check if hovering over diagram keyword
    let diagram_span = doc.diagram.span;
    let start = diagram_span.start();
    let end = diagram_span.end();
    if is_position_in_span(line, col, start.line(), start.col(), end.line(), end.col()) {
        // Check if we're on the "diagram" keyword itself (first 7 characters of the span)
        if line == start.line() && col >= start.col() && col < start.col() + 7 {
            return Some("The `diagram` section contains metadata about the diagram, such as version, width, color, and which box definition to use as the top-level box.".to_string());
        }
    }

    // Check if hovering over a box definition reference (identifier)
    if let Some(hover) = check_box_reference_hover(doc, text, line, col) {
        return Some(hover);
    }

    // Check box definitions and their contents
    for box_def in &doc.box_defs {
        if let Some(hover) = check_box_def_hover(box_def, text, line, col) {
            return Some(hover);
        }
    }

    None
}

/// Check if hovering over a property and return hover text
fn check_property_hover(
    props: &[diagramy::ast::Prop],
    _text: &str,
    line: usize,
    col: usize,
) -> Option<String> {
    use diagramy::ast::Prop;

    for prop in props {
        let span = prop.span();
        let start = span.start();
        let end = span.end();

        if !is_position_in_span(line, col, start.line(), start.col(), end.line(), end.col()) {
            continue;
        }

        // Get the property key
        let key = match prop {
            Prop::PropIdent { key, .. } => key,
            Prop::PropString { key, .. } => key,
            Prop::PropNumber { key, .. } => key,
            Prop::PropFrac { key, .. } => key,
            Prop::PropCoords { key, .. } => key,
            Prop::PropDim { key, .. } => key,
        };

        // Check if hovering over a Coords value
        if let Prop::PropCoords { value, .. } = prop {
            let coords_span = value.span;
            let coords_start = coords_span.start();
            let coords_end = coords_span.end();
            if is_position_in_span(line, col, coords_start.line(), coords_start.col(), coords_end.line(), coords_end.col()) {
                return Some("Coordinates specify a position as a (row, col) pair.\nBoth row and column are 1-based indices.".to_string());
            }
        }

        // Check if hovering over a Dim value
        if let Prop::PropDim { value, .. } = prop {
            let dim_span = value.span;
            let dim_start = dim_span.start();
            let dim_end = dim_span.end();
            if is_position_in_span(line, col, dim_start.line(), dim_start.col(), dim_end.line(), dim_end.col()) {
                return Some("Dimensions specify the size as HEIGHTxWIDTH.\nExample: 2x3 means 2 rows and 3 columns.".to_string());
            }
        }

        // Return property-specific help based on key
        return Some(get_property_help(key));
    }

    None
}

/// Get help text for a specific property key
fn get_property_help(key: &str) -> String {
    match key {
        "top" => "The `top` property names the box definition to be rendered as the top-level box.\nIf not specified, the first box definition in the file is used.".to_string(),
        "grid" => "The `grid` property specifies the number of rows and columns in this box.\nExample: grid: 3x4 means 3 rows and 4 columns.".to_string(),
        "color" => "The `color` property sets the background color of the box.".to_string(),
        "text" => "The `text` property sets the text content displayed in the box.\nMultiple strings can be provided for multi-line text.".to_string(),
        "width" => "The `width` property sets the width of the diagram in pixels.".to_string(),
        "version" => "The `version` property specifies the diagram format version.".to_string(),
        "borderStyle" => "The `borderStyle` property sets the border style: solid, dotted, dashed, or none.".to_string(),
        "margin" => "The `margin` property adjusts the padding around child boxes.\nValue is a multiplier of the default margin.".to_string(),
        _ => format!("Property: {}", key),
    }
}

/// Check if hovering over a box definition or its contents
fn check_box_def_hover(
    box_def: &diagramy::ast::BoxDef,
    text: &str,
    line: usize,
    col: usize,
) -> Option<String> {
    let span = box_def.span;
    let start = span.start();
    let end = span.end();

    if !is_position_in_span(line, col, start.line(), start.col(), end.line(), end.col()) {
        return None;
    }

    // Check if hovering over the box definition name
    let name_span = diagramy::ast::Span::from_offsets(text, box_def.name_location.0, box_def.name_location.1);
    let name_start = name_span.start();
    let name_end = name_span.end();

    if is_position_in_span(line, col, name_start.line(), name_start.col(), name_end.line(), name_end.col()) {
        return Some(format_box_def_info(box_def));
    }

    // Check if hovering over the "box" keyword at the start of a box definition
    // The "box" keyword should be at the start of the span
    if line == start.line() && col >= start.col() && col < start.col() + 3 {
        return Some("The `box` keyword is used to define a reusable box template or to place a box instance.".to_string());
    }

    // Check the box body for properties and box instances
    check_box_body_hover(&box_def.body, text, line, col)
}

/// Check if hovering over elements in a box body
fn check_box_body_hover(
    body: &diagramy::ast::BoxBody,
    text: &str,
    line: usize,
    col: usize,
) -> Option<String> {
    use diagramy::ast::BoxItem;

    for item in &body.items {
        match item {
            BoxItem::Prop(prop) => {
                if let Some(hover) = check_property_hover(&[prop.clone()], text, line, col) {
                    return Some(hover);
                }
            }
            BoxItem::BoxInst(inst) => {
                if let Some(hover) = check_box_inst_hover(inst, text, line, col) {
                    return Some(hover);
                }
            }
            BoxItem::Port(port) => {
                if let Some(hover) = check_port_hover(port, text, line, col) {
                    return Some(hover);
                }
            }
            BoxItem::Arrow(arrow) => {
                if let Some(hover) = check_arrow_hover(arrow, text, line, col) {
                    return Some(hover);
                }
            }
        }
    }

    None
}

/// Check if hovering over a box instance
fn check_box_inst_hover(
    inst: &diagramy::ast::BoxInst,
    text: &str,
    line: usize,
    col: usize,
) -> Option<String> {
    use diagramy::ast::BoxInst;

    let span = inst.span();
    let start = span.start();
    let end = span.end();

    if !is_position_in_span(line, col, start.line(), start.col(), end.line(), end.col()) {
        return None;
    }

    // Check if hovering over the "box" keyword at the start
    if line == start.line() && col >= start.col() && col < start.col() + 3 {
        return Some("The `box` keyword is used to define a reusable box template or to place a box instance.".to_string());
    }

    // Get coords and dim from the instance
    let (coords, dim) = match inst {
        BoxInst::WithBody { coords, dim, body, .. } => {
            // Check nested body recursively
            if let Some(hover) = check_box_body_hover(body, text, line, col) {
                return Some(hover);
            }
            (coords, dim)
        }
        BoxInst::Reference { coords, dim, .. } => (coords, dim),
    };

    // Check if hovering over coords
    if let Some(c) = coords {
        let coords_span = c.span;
        let coords_start = coords_span.start();
        let coords_end = coords_span.end();
        if is_position_in_span(line, col, coords_start.line(), coords_start.col(), coords_end.line(), coords_end.col()) {
            return Some("Coordinates specify a position as a (row, col) pair.\nBoth row and column are 1-based indices.".to_string());
        }

        // Check if hovering over "at" keyword (should be just before coords)
        // Approximate: "at" is typically 3 characters before the coords
        if line == coords_start.line() && col >= coords_start.col().saturating_sub(4) && col < coords_start.col() {
            return Some("The `at` keyword specifies the position where this box should be placed in the parent's grid.".to_string());
        }
    }

    // Check if hovering over dim
    let dim_span = dim.span;
    let dim_start = dim_span.start();
    let dim_end = dim_span.end();
    if is_position_in_span(line, col, dim_start.line(), dim_start.col(), dim_end.line(), dim_end.col()) {
        return Some("Dimensions specify the size as HEIGHTxWIDTH.\nExample: 2x3 means 2 rows and 3 columns.\nThis defines the bounding box into which the child box will be scaled.".to_string());
    }

    // Check if hovering over "dim" keyword (should be just before the dim value)
    if line == dim_start.line() && col >= dim_start.col().saturating_sub(5) && col < dim_start.col() {
        return Some("The `dim` keyword specifies the dimensions of the box being placed.\nIt defines the bounding box (in grid cells) into which the child box will be scaled.".to_string());
    }

    // Check if hovering over "is" keyword
    // The "is" keyword appears after "box" (and optional "at" and "dim")
    // We can approximate by checking the text around the expected position
    let line_text = text.lines().nth(line - 1)?;
    let col_idx = col.saturating_sub(1);
    if col_idx + 2 <= line_text.len() {
        let word = &line_text[col_idx..col_idx.min(col_idx + 2)];
        if word == "is" {
            return Some("The `is` keyword introduces the box's definition.\nThe definition may be given as the name of a box definition or inline inside curly braces.".to_string());
        }
    }

    None
}

/// Check if hovering over a port
fn check_port_hover(
    port: &diagramy::ast::Port,
    _text: &str,
    line: usize,
    col: usize,
) -> Option<String> {
    let span = port.span;
    let start = span.start();
    let end = span.end();

    if !is_position_in_span(line, col, start.line(), start.col(), end.line(), end.col()) {
        return None;
    }

    // Check if hovering over the "port" keyword
    if line == start.line() && col >= start.col() && col < start.col() + 4 {
        return Some("The `port` keyword defines a connection point on a box.\nPorts can be connected with arrows.".to_string());
    }

    // Check if hovering over the coords
    let coords_span = port.coords.span;
    let coords_start = coords_span.start();
    let coords_end = coords_span.end();
    if is_position_in_span(line, col, coords_start.line(), coords_start.col(), coords_end.line(), coords_end.col()) {
        return Some("Fractional coordinates specify a position as a (row, col) pair.\nValues range from (0.0, 0.0) at the upper-left corner to (grid_height, grid_width) at the lower-right corner.\nFor a 3x3 grid, (1.5, 1.5) would be the center of the box.".to_string());
    }

    // Check if hovering over "at" keyword
    if line == coords_start.line() && col >= coords_start.col().saturating_sub(4) && col < coords_start.col() {
        return Some("The `at` keyword specifies the position of the port on the box.".to_string());
    }

    None
}

/// Check if hovering over an arrow
fn check_arrow_hover(
    arrow: &diagramy::ast::Arrow,
    text: &str,
    line: usize,
    col: usize,
) -> Option<String> {
    let span = arrow.span;
    let start = span.start();
    let end = span.end();

    if !is_position_in_span(line, col, start.line(), start.col(), end.line(), end.col()) {
        return None;
    }

    // Check if hovering over the "arrow" keyword
    if line == start.line() && col >= start.col() && col < start.col() + 5 {
        return Some("The `arrow` keyword defines a connection between two ports.\nArrows are rendered with arrowheads pointing to the destination port.".to_string());
    }

    // Check if hovering over "to" keyword
    // This is approximate - we look for "to" in the line
    let line_text = text.lines().nth(line - 1)?;
    if let Some(to_pos) = line_text.find(" to ") {
        let to_col = to_pos + 2; // +1 for 0-based to 1-based, +1 for space before "to"
        if col >= to_col && col < to_col + 2 {
            return Some("The `to` keyword specifies the destination port for the arrow.".to_string());
        }
    }

    None
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: Arc::new(RwLock::new(HashMap::new())),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
