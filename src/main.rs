#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::error::Error;

use lsp_server::{Connection, ExtractError, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{DidCloseTextDocument, DidOpenTextDocument};
use lsp_types::CompletionResponse;
use lsp_types::{request::Completion, InitializeParams, ServerCapabilities};
use lsp_types::{CompletionItemKind, CompletionOptions};

mod snippets;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    // Note that  we must have our logging only write out to stderr.
    eprintln!("starting snippets server");

    // Create the transport. Includes the stdio (stdin and stdout) versions but this could
    // also be implemented to use sockets or HTTP.
    let (connection, io_threads) = Connection::stdio();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        completion_provider: Some(CompletionOptions::default()),
        ..Default::default()
    })
    .unwrap();

    let initialization_params = connection.initialize(server_capabilities)?;
    let initialization_params: InitializeParams = serde_json::from_value(initialization_params)?;

    main_loop(connection, initialization_params)?;
    io_threads.join()?;

    // Shut down gracefully.
    eprintln!("shutting down server");
    Ok(())
}

fn main_loop(
    connection: Connection,
    params: InitializeParams,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let snippets = snippets::parse(params);
    let mut state = HashMap::new();

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                match cast::<Completion>(req) {
                    Ok((id, params)) => {
                        eprintln!("got completion request #{id}: {params:?}");

                        let completion_items = if let Some(lang) =
                            state.get(&params.text_document_position.text_document.uri)
                        {
                            if let Some(lang_snippets) = snippets.get(lang) {
                                let mut items = Vec::new();

                                for (key, value) in lang_snippets.into_iter() {
                                    items.push(lsp_types::CompletionItem {
                                        label: key.to_owned(),
                                        kind: Some(CompletionItemKind::SNIPPET),
                                        insert_text: Some(value.to_owned()),
                                        ..Default::default()
                                    });
                                }
                                items
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };

                        let result = Some(CompletionResponse::Array(completion_items));
                        let result = serde_json::to_value(&result).unwrap();
                        let resp = Response {
                            id,
                            result: Some(result),
                            error: None,
                        };
                        connection.sender.send(Message::Response(resp))?;
                        continue;
                    }
                    Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                    Err(ExtractError::MethodMismatch(req)) => req,
                };
            }
            Message::Response(resp) => {
                eprintln!("got response: {resp:?}");
            }
            Message::Notification(not) => {
                let not = match cast_notification::<DidOpenTextDocument>(not) {
                    Ok(params) => {
                        state.insert(params.text_document.uri, params.text_document.language_id);

                        continue;
                    }
                    Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                    Err(ExtractError::MethodMismatch(not)) => not,
                };

                match cast_notification::<DidCloseTextDocument>(not) {
                    Ok(params) => {
                        state.remove(&params.text_document.uri);

                        continue;
                    }
                    Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                    Err(ExtractError::MethodMismatch(not)) => not,
                };
            }
        }
    }
    Ok(())
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}

fn cast_notification<N>(not: Notification) -> Result<N::Params, ExtractError<Notification>>
where
    N: lsp_types::notification::Notification,
    N::Params: serde::de::DeserializeOwned,
{
    not.extract(N::METHOD)
}
