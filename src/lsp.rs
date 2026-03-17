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
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents.write().await.insert(uri, text);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents.write().await.insert(uri, change.text);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.write().await.remove(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let _uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        self.client
            .log_message(
                MessageType::INFO,
                format!("Hover request at {}:{}", position.line, position.character),
            )
            .await;

        // TODO: Parse the document and provide actual hover information
        // For now, return a simple placeholder
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(
                "Diagramy language element".to_string(),
            )),
            range: None,
        }))
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
