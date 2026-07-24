// TODO(mfeist)
//
// - Get workspace folders. Not sure if the protocol specifies that root is in
// workspace folders or not.
//
// - Trigger find_tags when we get the initial folders and on any updates.
//
// - Get file updates.
//
// - Provide document link and document link resolution.

use std::error::Error;

use lsp_types::{
    ChangeNotifications, Diagnostic, DiagnosticSeverity, DidChangeTextDocumentNotification,
    DidChangeTextDocumentParams, DidChangeWorkspaceFoldersNotification,
    DidChangeWorkspaceFoldersParams, DidOpenTextDocumentNotification, DidOpenTextDocumentParams,
    DocumentLink, DocumentLinkOptions, DocumentLinkRequest, InitializeParams,
    LspNotificationMethod, LspRequestMethod, Notification, Position,
    PublishDiagnosticsNotification, PublishDiagnosticsParams, Range, Request, ServerCapabilities,
    TextDocumentSync, Uri, WorkDoneProgressOptions, WorkspaceFolders,
    WorkspaceFoldersServerCapabilities, WorkspaceOptions,
};
use rustc_hash::FxHashMap; // fast hash map

#[allow(
    clippy::print_stderr,
    clippy::disallowed_types,
    clippy::disallowed_methods
)]
use anyhow::Result;
use lsp_server::{
    Connection, Message, Request as ServerRequest, RequestId, Response, ResponseKind,
};

// =====================================================================
// main
// =====================================================================

#[allow(clippy::print_stderr)]
fn main() -> std::result::Result<(), Box<dyn Error + Sync + Send>> {
    env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("info")).init();
    log::info!("Starting traversal-lsp");

    // transport
    let (connection, io_thread) = Connection::stdio();

    // advertised capabilities
    let caps = ServerCapabilities {
        text_document_sync: Some(TextDocumentSync::Kind(
            lsp_types::TextDocumentSyncKind::Full,
        )),
        document_link_provider: Some(DocumentLinkOptions::new(
            Some(false),
            WorkDoneProgressOptions::new(Some(false)),
        )),
        workspace: Some(WorkspaceOptions::new(
            Some(WorkspaceFoldersServerCapabilities::new(
                Some(true),
                Some(ChangeNotifications::Bool(true)),
            )),
            None,
            None,
        )),
        ..Default::default()
    };
    let init_value = serde_json::json!({
        "capabilities": caps,
        "offsetEncoding": ["utf-8"],
    });

    let init_params = connection.initialize(init_value)?;
    main_loop(connection, init_params)?;
    io_thread.join()?;
    log::info!("shutting down server");
    Ok(())
}

// =====================================================================
// event loop
// =====================================================================

fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> std::result::Result<(), Box<dyn Error + Sync + Send>> {
    let init: InitializeParams = serde_json::from_value(params)?;
    let mut docs: FxHashMap<Uri, String> = FxHashMap::default();

    if let Some(workspace_folders) = init.workspace_folders_initialize_params.workspace_folders {
        if let WorkspaceFolders::WorkspaceFolderList(workspace_folders_list) = workspace_folders {
            for folder in workspace_folders_list {
                log::info!("{}", folder.uri);
            }
        }
    }

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }
                if let Err(err) = handle_request(&connection, &req, &mut docs) {
                    log::error!("[lsp] request {} failed: {err}", req.method);
                }
            }
            Message::Notification(note) => {
                if let Err(err) = handle_notification(&connection, &note, &mut docs) {
                    log::error!("[lsp] notification {} failed: {err}", note.method);
                }
            }
            Message::Response(resp) => log::error!("[lsp] response: {resp:?}"),
        }
    }
    Ok(())
}

// =====================================================================
// notifications
// =====================================================================

fn handle_notification(
    conn: &Connection,
    note: &lsp_server::Notification,
    docs: &mut FxHashMap<Uri, String>,
) -> Result<()> {
    let method: LspNotificationMethod<'_> = note.method.as_str().into();
    match method {
        DidOpenTextDocumentNotification::METHOD => {
            let p: DidOpenTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = p.text_document.uri;
            docs.insert(uri.clone(), p.text_document.text);
            publish_dummy_diag(conn, &uri)?;
        }
        DidChangeTextDocumentNotification::METHOD => {
            let p: DidChangeTextDocumentParams = serde_json::from_value(note.params.clone())?;
            if let Some(change) = p.content_changes.into_iter().next() {
                let uri = p.text_document.text_document_identifier.uri;
                let text = match change {
                    lsp_types::TextDocumentContentChangeEvent::TextDocumentContentChangePartial(partial) => partial.text,
                    lsp_types::TextDocumentContentChangeEvent::TextDocumentContentChangeWholeDocument(whole) => whole.text,
                };
                docs.insert(uri.clone(), text);
                publish_dummy_diag(conn, &uri)?;
            }
        }
        DidChangeWorkspaceFoldersNotification::METHOD => {
            let p: DidChangeWorkspaceFoldersParams = serde_json::from_value(note.params.clone())?;
            for added in &p.event.added {
                log::info!(
                    "[lsp] Added workspace folder '{}': {}",
                    added.name,
                    added.uri
                );
            }
            for removed in &p.event.removed {
                log::info!(
                    "[lsp] Removed workspace folder '{}': {}",
                    removed.name,
                    removed.uri
                );
            }
        }
        _ => {}
    }
    Ok(())
}

// =====================================================================
// requests
// =====================================================================

fn handle_request(
    conn: &Connection,
    req: &ServerRequest,
    docs: &mut FxHashMap<Uri, String>,
) -> Result<()> {
    let parsed: LspRequestMethod<'_> = req.method.as_str().into();
    match parsed {
        DocumentLinkRequest::METHOD => {
            let document_links = Vec::<DocumentLink>::new();
            send_ok(conn, req.id.clone(), &document_links)?;
        }
        // CompletionRequest::METHOD => {
        //     let item = CompletionItem {
        //         label: "HelloFromLSP".into(),
        //         kind: Some(CompletionItemKind::Function),
        //         detail: Some("dummy completion".into()),
        //         ..Default::default()
        //     };
        //     let items = vec![item];
        //     let completion_list = CompletionResponse::CompletionList(lsp_types::CompletionList {
        //         is_incomplete: false,
        //         item_defaults: None,
        //         apply_kind: None,
        //         items,
        //     });
        //     send_ok(conn, req.id.clone(), &completion_list)?;
        // }
        _ => send_err(
            conn,
            req.id.clone(),
            lsp_server::ErrorCode::MethodNotFound,
            "unhandled method",
        )?,
    }
    Ok(())
}

// =====================================================================
// diagnostics
// =====================================================================
fn publish_dummy_diag(conn: &Connection, uri: &Uri) -> Result<()> {
    let diag = Diagnostic {
        range: Range::new(Position::new(0, 0), Position::new(0, 1)),
        severity: Some(DiagnosticSeverity::Information),
        code: None,
        code_description: None,
        source: Some("minimal_lsp".into()),
        message: "dummy diagnostic".into(),
        related_information: None,
        tags: None,
        data: None,
    };
    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics: vec![diag],
        version: None,
    };
    conn.sender
        .send(Message::Notification(lsp_server::Notification::new(
            PublishDiagnosticsNotification::METHOD.into(),
            params,
        )))?;
    Ok(())
}

// =====================================================================
// helpers
// =====================================================================

fn full_range(text: &str) -> Range {
    let last_line_idx = text.lines().count().saturating_sub(1) as u32;
    let last_col = text.lines().last().map_or(0, |l| l.chars().count()) as u32;
    Range::new(Position::new(0, 0), Position::new(last_line_idx, last_col))
}

fn send_ok<T: serde::Serialize>(conn: &Connection, id: RequestId, result: &T) -> Result<()> {
    let resp = Response {
        id,
        response_kind: ResponseKind::Ok {
            result: serde_json::to_value(result)?,
        },
    };
    conn.sender.send(Message::Response(resp))?;
    Ok(())
}

fn send_err(
    conn: &Connection,
    id: RequestId,
    code: lsp_server::ErrorCode,
    msg: &str,
) -> Result<()> {
    let resp = Response {
        id,
        response_kind: ResponseKind::Err {
            error: lsp_server::ResponseError {
                code: code as i32,
                message: msg.into(),
                data: None,
            },
        },
    };
    conn.sender.send(Message::Response(resp))?;
    Ok(())
}
