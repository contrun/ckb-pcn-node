mod config;
pub use config::RpcConfig;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use log::{debug, error, info};
use serde::Deserialize;
use std::{future::Future, sync::Arc};
use tokio::sync::mpsc;

pub type NetworkActorCommandWithReply =
    (NetworkActorCommand, Option<mpsc::Sender<crate::Result<()>>>);

use crate::{cch::CchCommand, ckb::NetworkActorCommand};

#[derive(Debug, Deserialize)]
pub struct HttpBody<T> {
    pub id: Option<u64>,
    pub request: T,
}

type CkbRpcRequest = HttpBody<NetworkActorCommand>;
type CchRpcRequest = HttpBody<CchCommand>;

pub struct CkbRpcState {
    pub ckb_command_sender: mpsc::Sender<NetworkActorCommandWithReply>,
}

pub struct CchRpcState {
    pub cch_command_sender: mpsc::Sender<CchCommand>,
}

async fn serve_ckb_rpc(
    State(state): State<Arc<CkbRpcState>>,
    Json(http_request): Json<CkbRpcRequest>,
) -> impl IntoResponse {
    debug!("Received http request: {:?}", http_request);
    let (sender, mut receiver) = mpsc::channel(1);
    let command = (http_request.request, Some(sender));
    state
        .ckb_command_sender
        .send(command)
        .await
        .expect("send command");
    match receiver.recv().await {
        Some(Ok(_)) => StatusCode::OK,
        Some(Err(err)) => {
            error!("Error processing command: {:?}", err);
            StatusCode::BAD_REQUEST
        }
        None => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn serve_cch_rpc(
    State(state): State<Arc<CchRpcState>>,
    Json(http_request): Json<CchRpcRequest>,
) -> impl IntoResponse {
    debug!("Received http request: {:?}", http_request);
    state
        .cch_command_sender
        .send(http_request.request)
        .await
        .expect("send command");
    StatusCode::OK
}

pub async fn start_rpc<F>(
    config: RpcConfig,
    ckb_command_sender: Option<mpsc::Sender<NetworkActorCommandWithReply>>,
    cch_command_sender: Option<mpsc::Sender<CchCommand>>,
    shutdown_signal: F,
) where
    F: Future<Output = ()> + Send + 'static,
{
    let mut app = Router::new();
    if let Some(ckb_command_sender) = ckb_command_sender {
        let ckb_router = Router::new()
            .route("/", post(serve_ckb_rpc))
            .with_state(Arc::new(CkbRpcState { ckb_command_sender }));
        app = app.nest("/ckb", ckb_router);
    }
    if let Some(cch_command_sender) = cch_command_sender {
        let cch_router = Router::new()
            .route("/", post(serve_cch_rpc))
            .with_state(Arc::new(CchRpcState { cch_command_sender }));
        app = app.nest("/cch", cch_router);
    }

    let listening_addr = config.listening_addr.as_deref().unwrap_or("[::]:0");
    let listener = tokio::net::TcpListener::bind(listening_addr)
        .await
        .expect("bind rpc addr");
    info!("Starting rpc server at {:?}", &listener.local_addr());
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .expect("start rpc server");
    info!("Rpc service stopped");
}
