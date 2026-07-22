use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::extract::Extension;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;
use metrics_exporter_prometheus::PrometheusHandle;
use scanengine_api::ScanSchema;

async fn graphql_handler(schema: Extension<ScanSchema>, req: GraphQLRequest) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn playground() -> impl IntoResponse {
    Html(playground_source(
        GraphQLPlaygroundConfig::new("/graphql").subscription_endpoint("/graphql/ws"),
    ))
}

async fn health_live() -> impl IntoResponse {
    "OK"
}

async fn health_ready() -> impl IntoResponse {
    "READY"
}

async fn metrics_handler(Extension(handle): Extension<PrometheusHandle>) -> impl IntoResponse {
    handle.render()
}

/// Assemble the axum application router.
pub fn build_app(schema: ScanSchema, metrics: PrometheusHandle) -> Router {
    Router::new()
        .route("/graphql", get(playground).post(graphql_handler))
        .route_service("/graphql/ws", GraphQLSubscription::new(schema.clone()))
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/metrics", get(metrics_handler))
        .layer(Extension(metrics))
        .layer(Extension(schema))
}

/// Await a shutdown signal (Ctrl-C or SIGTERM).
pub async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use scanengine_api::build_schema;
    use scanengine_core::{ScanConfig, ScanEngine};
    use scanengine_infra::{BroadcastSignalBus, InMemoryRuleStore};
    use std::sync::Arc;
    use tower::ServiceExt;

    fn app() -> Router {
        let store = Arc::new(InMemoryRuleStore::new());
        let bus = Arc::new(BroadcastSignalBus::new(256));
        let engine = Arc::new(ScanEngine::new(ScanConfig::default(), store, bus));
        let schema = build_schema(engine);
        let handle = crate::telemetry::build_metrics_handle();
        build_app(schema, handle)
    }

    #[tokio::test]
    async fn health_ready_returns_ok() {
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/health/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn metrics_endpoint_responds() {
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
