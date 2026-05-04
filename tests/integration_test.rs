//! 集成测试 - 验证多模块协作的完整流程
//!
//! 测试范围：
//! - HTTP 认证中间件层（真实 axum Router）
//! - LightRAG 客户端 + mock 服务集成
//! - MCP 服务器权限矩阵（3 用户 × 4 工具）

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use pangenmcp::auth::UserContext;
use pangenmcp::config::{
    AuthConfig, Config, DefaultsConfig, LightRagConfig, McpConfig, ServerConfig, TokenConfig,
};
use pangenmcp::http::build_app;
use pangenmcp::mcp::{McpServer, SharedState};
use std::sync::Arc;
use tower::ServiceExt;

// ============================================================
// 测试辅助函数
// ============================================================

fn build_test_config(lightrag_url: &str) -> Config {
    Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
        },
        mcp: McpConfig {
            server_name: "test-server".to_string(),
            version: "0.1.0-test".to_string(),
        },
        auth: AuthConfig {
            tokens: vec![
                TokenConfig {
                    name: "alice".to_string(),
                    token: "alice-token".to_string(),
                    scopes: vec!["rag:read".to_string()],
                },
                TokenConfig {
                    name: "bob".to_string(),
                    token: "bob-token".to_string(),
                    scopes: vec!["rag:read".to_string(), "rag:write".to_string()],
                },
                TokenConfig {
                    name: "admin".to_string(),
                    token: "admin-token".to_string(),
                    scopes: vec![
                        "rag:read".to_string(),
                        "rag:write".to_string(),
                        "rag:admin".to_string(),
                    ],
                },
            ],
            audit_log_path: "test_integration_audit.log".to_string(),
        },
        lightrag: LightRagConfig {
            url: lightrag_url.to_string(),
            timeout_seconds: 5,
            max_retries: 1,
            retry_delay_seconds: 0,
        },
        defaults: DefaultsConfig {
            query_mode: "hybrid".to_string(),
            top_k: 10,
            response_type: "Multiple Paragraphs".to_string(),
        },
    }
}

fn user_context(name: &str, scopes: &[&str]) -> UserContext {
    UserContext {
        name: name.to_string(),
        scopes: scopes.iter().map(|s| s.to_string()).collect(),
    }
}

// ============================================================
// 1. HTTP 认证集成测试（真实 axum Router）
// ============================================================

#[tokio::test]
async fn test_http_no_token_returns_unauthorized() {
    let config = build_test_config("http://localhost:9999");
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // AppError::Auth maps to UNAUTHORIZED
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    // RFC 6750: 应当包含 WWW-Authenticate: Bearer，明确告知客户端使用静态 Bearer
    let www_auth = response.headers().get(header::WWW_AUTHENTICATE);
    assert!(www_auth.is_some());
    assert!(www_auth.unwrap().to_str().unwrap().starts_with("Bearer"));
}

#[tokio::test]
async fn test_non_mcp_path_returns_not_found_without_auth() {
    // /.well-known/* 等非 /mcp 路径不应当被认证中间件拦截，
    // 应直接返回 404，避免触发 MCP 客户端的 OAuth 自动协商。
    let config = build_test_config("http://localhost:9999");
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/.well-known/oauth-protected-resource")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_register_endpoint_returns_not_found() {
    // OAuth 动态注册端点不存在，应返回 404 而非 401
    let config = build_test_config("http://localhost:9999");
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/register")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_http_invalid_bearer_format_returns_unauthorized() {
    let config = build_test_config("http://localhost:9999");
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header(header::AUTHORIZATION, "Basic dXNlcjpwYXNz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_http_invalid_token_returns_unauthorized() {
    let config = build_test_config("http://localhost:9999");
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header(header::AUTHORIZATION, "Bearer not-a-real-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_http_valid_token_passes_auth_layer() {
    let config = build_test_config("http://localhost:9999");
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header(header::AUTHORIZATION, "Bearer alice-token")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .body(Body::from(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // 通过认证层后到达 MCP 处理器，不再返回 401
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_http_empty_bearer_token_returns_unauthorized() {
    let config = build_test_config("http://localhost:9999");
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header(header::AUTHORIZATION, "Bearer ")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================
// 2. RAG 客户端 + mock LightRAG 集成测试
// ============================================================

#[tokio::test]
async fn test_rag_client_query_full_flow() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("POST", "/query")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"response":"integration test result"}"#)
        .create_async()
        .await;

    let config = build_test_config(&server.url());
    let shared = Arc::new(SharedState::new(&config));

    let request = pangenmcp::rag::QueryRequest {
        query: "test query".to_string(),
        mode: "hybrid".to_string(),
        top_k: 10,
        response_type: "Multiple Paragraphs".to_string(),
    };

    let result = shared.rag_client.query(request).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.response, "integration test result");
}

#[tokio::test]
async fn test_rag_client_insert_full_flow() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("POST", "/documents/text")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"status":"success","message":"document inserted"}"#)
        .create_async()
        .await;

    let config = build_test_config(&server.url());
    let shared = Arc::new(SharedState::new(&config));

    let request = pangenmcp::rag::InsertRequest {
        text: "test document".to_string(),
        description: None,
    };

    let result = shared.rag_client.insert(request).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, "success");
}

#[tokio::test]
async fn test_rag_client_clear_full_flow() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("DELETE", "/documents")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"status":"success","message":"all documents cleared"}"#)
        .create_async()
        .await;

    let config = build_test_config(&server.url());
    let shared = Arc::new(SharedState::new(&config));

    let result = shared.rag_client.clear().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, "success");
}

#[tokio::test]
async fn test_rag_client_health_full_flow() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("GET", "/health")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"status":"healthy","working_directory":"/data","configuration":{"llm_model":"gpt-4","embedding_model":"text-embedding-3"}}"#,
        )
        .create_async()
        .await;

    let config = build_test_config(&server.url());
    let shared = Arc::new(SharedState::new(&config));

    let result = shared.rag_client.health().await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, "healthy");
    assert_eq!(response.configuration.llm_model, "gpt-4");
}

#[tokio::test]
async fn test_rag_client_unreachable_returns_error() {
    // 不启动 mock 服务器，直接连接到不可达地址
    let config = build_test_config("http://127.0.0.1:1");
    let shared = Arc::new(SharedState::new(&config));

    let result = shared.rag_client.health().await;
    assert!(result.is_err());
}

// ============================================================
// 3. 权限矩阵集成测试（3 用户 × 4 工具 = 12 个场景）
//
// 测试矩阵：
//   工具         | Alice (read) | Bob (read+write) | Admin (all)
//   rag_query   |   ✓          |   ✓              |   ✓
//   rag_insert  |   ✗          |   ✓              |   ✓
//   rag_clear   |   ✗          |   ✓              |   ✓
//   rag_health  |   ✗          |   ✗              |   ✓
// ============================================================

fn make_test_server() -> McpServer {
    let config = build_test_config("http://localhost:9999");
    let shared = Arc::new(SharedState::new(&config));
    McpServer::new(shared)
}

// --- Alice (rag:read) ---

#[tokio::test]
async fn test_permission_matrix_alice_rag_query() {
    let server = make_test_server();
    let user = user_context("alice", &["rag:read"]);
    assert!(server.check_scope(&user, "rag:read").is_ok());
}

#[tokio::test]
async fn test_permission_matrix_alice_rag_insert() {
    let server = make_test_server();
    let user = user_context("alice", &["rag:read"]);
    assert!(server.check_scope(&user, "rag:write").is_err());
}

#[tokio::test]
async fn test_permission_matrix_alice_rag_clear() {
    let server = make_test_server();
    let user = user_context("alice", &["rag:read"]);
    assert!(server.check_scope(&user, "rag:write").is_err());
}

#[tokio::test]
async fn test_permission_matrix_alice_rag_health() {
    let server = make_test_server();
    let user = user_context("alice", &["rag:read"]);
    assert!(server.check_scope(&user, "rag:admin").is_err());
}

// --- Bob (rag:read + rag:write) ---

#[tokio::test]
async fn test_permission_matrix_bob_rag_query() {
    let server = make_test_server();
    let user = user_context("bob", &["rag:read", "rag:write"]);
    assert!(server.check_scope(&user, "rag:read").is_ok());
}

#[tokio::test]
async fn test_permission_matrix_bob_rag_insert() {
    let server = make_test_server();
    let user = user_context("bob", &["rag:read", "rag:write"]);
    assert!(server.check_scope(&user, "rag:write").is_ok());
}

#[tokio::test]
async fn test_permission_matrix_bob_rag_clear() {
    let server = make_test_server();
    let user = user_context("bob", &["rag:read", "rag:write"]);
    assert!(server.check_scope(&user, "rag:write").is_ok());
}

#[tokio::test]
async fn test_permission_matrix_bob_rag_health() {
    let server = make_test_server();
    let user = user_context("bob", &["rag:read", "rag:write"]);
    assert!(server.check_scope(&user, "rag:admin").is_err());
}

// --- Admin (all scopes) ---

#[tokio::test]
async fn test_permission_matrix_admin_rag_query() {
    let server = make_test_server();
    let user = user_context("admin", &["rag:read", "rag:write", "rag:admin"]);
    assert!(server.check_scope(&user, "rag:read").is_ok());
}

#[tokio::test]
async fn test_permission_matrix_admin_rag_insert() {
    let server = make_test_server();
    let user = user_context("admin", &["rag:read", "rag:write", "rag:admin"]);
    assert!(server.check_scope(&user, "rag:write").is_ok());
}

#[tokio::test]
async fn test_permission_matrix_admin_rag_clear() {
    let server = make_test_server();
    let user = user_context("admin", &["rag:read", "rag:write", "rag:admin"]);
    assert!(server.check_scope(&user, "rag:write").is_ok());
}

#[tokio::test]
async fn test_permission_matrix_admin_rag_health() {
    let server = make_test_server();
    let user = user_context("admin", &["rag:read", "rag:write", "rag:admin"]);
    assert!(server.check_scope(&user, "rag:admin").is_ok());
}

// ============================================================
// 4. 管理 API 集成测试
// ============================================================

// --- 迭代 1: GET /api/health (无需认证) ---

#[tokio::test]
async fn test_api_health_accessible_without_auth() {
    let config = build_test_config("http://localhost:9999");
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // /api/health 不需要认证，应该返回 200（即使 LightRAG 不可达）
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_health_returns_server_info() {
    let config = build_test_config("http://localhost:9999");
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // 服务器信息
    assert_eq!(json["server"]["status"], "healthy");
    assert!(json["server"]["version"].is_string());

    // LightRAG 信息字段存在（状态可能是 unreachable，因为 mock url 不可达）
    assert!(json["lightrag"]["url"].is_string());
    assert!(json["lightrag"]["status"].is_string());
}

#[tokio::test]
async fn test_api_health_reports_lightrag_unreachable() {
    // LightRAG 不可达时，应当返回状态为 unreachable 而非 500
    let config = build_test_config("http://127.0.0.1:1");  // 不可达端口
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // 服务器仍然健康
    assert_eq!(json["server"]["status"], "healthy");
    // LightRAG 不可达
    assert_eq!(json["lightrag"]["status"], "unreachable");
}

// --- 迭代 2: GET /api/stats (需要 stats:read scope) ---

#[tokio::test]
async fn test_api_stats_requires_auth() {
    let config = build_test_config("http://localhost:9999");
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/stats")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_stats_rejects_token_without_stats_read_scope() {
    // alice 只有 rag:read，没有 stats:read，应被拒绝
    let config = build_test_config("http://localhost:9999");
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/stats")
        .header(header::AUTHORIZATION, "Bearer alice-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_api_stats_returns_empty_initially() {
    // 带 stats:read scope 的 admin 可以访问，初始无请求记录
    let config = build_test_config_with_admin_stats();
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/stats")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["total_requests"], 0);
    assert_eq!(json["total_errors"], 0);
    assert!(json["uptime_seconds"].is_number());
    assert!(json["by_tool"].is_object());
}

fn build_test_config_with_admin_stats() -> Config {
    let mut config = build_test_config("http://localhost:9999");
    // 给 admin 加上 stats:read scope
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("stats:read".to_string());
    }
    config
}

#[tokio::test]
async fn test_stats_records_tool_invocation() {
    // 调用 SharedState 的 stats 接口直接验证记录逻辑
    // （工具方法记录的回归在单元测试中已覆盖；这里验证 API 暴露的快照正确）
    let config = build_test_config_with_admin_stats();
    let shared = Arc::new(SharedState::new(&config));

    // 直接记录一次（模拟工具执行）
    shared.stats.write().await.record("rag_query", 123.0, true);
    shared.stats.write().await.record("rag_query", 456.0, false);

    let snap = shared.stats.read().await.snapshot();
    assert_eq!(snap.total_requests, 2);
    assert_eq!(snap.total_errors, 1);
    let tool = snap.by_tool.get("rag_query").expect("rag_query stats");
    assert_eq!(tool.requests, 2);
    assert_eq!(tool.errors, 1);
}
