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

/// 创建测试用的 app（包含 metrics 初始化）
fn build_test_app(config: &Config) -> axum::Router {
    let metrics_handle = pangenmcp::metrics::init_metrics();
    pangenmcp::metrics::register_metrics();
    build_app(Arc::new(SharedState::new(config)), metrics_handle)
}

// ============================================================
// 1. HTTP 认证集成测试（真实 axum Router）
// ============================================================

#[tokio::test]
async fn test_http_no_token_returns_unauthorized() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

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
    let app = build_test_app(&config);

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
    let app = build_test_app(&config);

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
    let app = build_test_app(&config);

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
    let app = build_test_app(&config);

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
    let app = build_test_app(&config);

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
    let app = build_test_app(&config);

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
    assert!(server.check_scope(&user, "rag:read").await.is_ok());
}

#[tokio::test]
async fn test_permission_matrix_alice_rag_insert() {
    let server = make_test_server();
    let user = user_context("alice", &["rag:read"]);
    assert!(server.check_scope(&user, "rag:write").await.is_err());
}

#[tokio::test]
async fn test_permission_matrix_alice_rag_clear() {
    let server = make_test_server();
    let user = user_context("alice", &["rag:read"]);
    assert!(server.check_scope(&user, "rag:write").await.is_err());
}

#[tokio::test]
async fn test_permission_matrix_alice_rag_health() {
    let server = make_test_server();
    let user = user_context("alice", &["rag:read"]);
    assert!(server.check_scope(&user, "rag:admin").await.is_err());
}

// --- Bob (rag:read + rag:write) ---

#[tokio::test]
async fn test_permission_matrix_bob_rag_query() {
    let server = make_test_server();
    let user = user_context("bob", &["rag:read", "rag:write"]);
    assert!(server.check_scope(&user, "rag:read").await.is_ok());
}

#[tokio::test]
async fn test_permission_matrix_bob_rag_insert() {
    let server = make_test_server();
    let user = user_context("bob", &["rag:read", "rag:write"]);
    assert!(server.check_scope(&user, "rag:write").await.is_ok());
}

#[tokio::test]
async fn test_permission_matrix_bob_rag_clear() {
    let server = make_test_server();
    let user = user_context("bob", &["rag:read", "rag:write"]);
    assert!(server.check_scope(&user, "rag:write").await.is_ok());
}

#[tokio::test]
async fn test_permission_matrix_bob_rag_health() {
    let server = make_test_server();
    let user = user_context("bob", &["rag:read", "rag:write"]);
    assert!(server.check_scope(&user, "rag:admin").await.is_err());
}

// --- Admin (all scopes) ---

#[tokio::test]
async fn test_permission_matrix_admin_rag_query() {
    let server = make_test_server();
    let user = user_context("admin", &["rag:read", "rag:write", "rag:admin"]);
    assert!(server.check_scope(&user, "rag:read").await.is_ok());
}

#[tokio::test]
async fn test_permission_matrix_admin_rag_insert() {
    let server = make_test_server();
    let user = user_context("admin", &["rag:read", "rag:write", "rag:admin"]);
    assert!(server.check_scope(&user, "rag:write").await.is_ok());
}

#[tokio::test]
async fn test_permission_matrix_admin_rag_clear() {
    let server = make_test_server();
    let user = user_context("admin", &["rag:read", "rag:write", "rag:admin"]);
    assert!(server.check_scope(&user, "rag:write").await.is_ok());
}

#[tokio::test]
async fn test_permission_matrix_admin_rag_health() {
    let server = make_test_server();
    let user = user_context("admin", &["rag:read", "rag:write", "rag:admin"]);
    assert!(server.check_scope(&user, "rag:admin").await.is_ok());
}

// ============================================================
// 4. 管理 API 集成测试
// ============================================================

// --- 迭代 1: GET /api/health (无需认证) ---

#[tokio::test]
async fn test_api_health_accessible_without_auth() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

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
    let app = build_test_app(&config);

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
    let app = build_test_app(&config);

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
    let app = build_test_app(&config);

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
    let app = build_test_app(&config);

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
    let app = build_test_app(&config);

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

// --- 迭代 3: GET /api/config (需要 config:read scope) ---

#[tokio::test]
async fn test_api_config_requires_auth() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/config")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_config_rejects_token_without_config_read_scope() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/config")
        .header(header::AUTHORIZATION, "Bearer alice-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_api_config_masks_tokens() {
    let mut config = build_test_config("http://localhost:9999");
    // 给 admin 加上 config:read scope
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("config:read".to_string());
    }
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/config")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // 验证结构存在
    assert!(json["server"].is_object());
    assert!(json["auth"].is_object());
    assert!(json["lightrag"].is_object());
    assert!(json["defaults"].is_object());

    // 验证 token 被脱敏
    let tokens = json["auth"]["tokens"].as_array().expect("tokens array");
    for token_obj in tokens {
        let token_value = token_obj["token"].as_str().expect("token string");
        assert_eq!(token_value, "***", "token should be masked");
    }
}

#[tokio::test]
async fn test_api_config_returns_complete_structure() {
    let mut config = build_test_config("http://localhost:9999");
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("config:read".to_string());
    }
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/config")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // 验证关键字段
    assert_eq!(json["server"]["host"], "127.0.0.1");
    assert_eq!(json["server"]["port"], 0);
    assert_eq!(json["lightrag"]["url"], "http://localhost:9999");
    assert_eq!(json["defaults"]["query_mode"], "hybrid");
    assert_eq!(json["defaults"]["top_k"], 10);
}

// --- 迭代 4: PATCH /api/config (需要 config:write scope) ---

#[tokio::test]
async fn test_api_config_patch_requires_auth() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::PATCH)
        .uri("/api/config")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"defaults":{"query_mode":"hybrid","top_k":20,"response_type":"simple"}}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_config_patch_rejects_token_without_config_write_scope() {
    let mut config = build_test_config("http://localhost:9999");
    // admin 只有 config:read，没有 config:write
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("config:read".to_string());
    }
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::PATCH)
        .uri("/api/config")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"defaults":{"query_mode":"hybrid","top_k":20,"response_type":"simple"}}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_api_config_patch_updates_defaults() {
    use tempfile::NamedTempFile;
    use std::io::Write;

    // 创建临时配置文件
    let mut temp_file = NamedTempFile::new().unwrap();
    let config_content = r#"
[server]
host = "127.0.0.1"
port = 8080

[mcp]
server_name = "test-server"
version = "1.0.0"

[auth]
audit_log_path = "./audit.log"

[[auth.tokens]]
name = "admin"
token = "admin-token"
scopes = ["rag:read", "rag:write", "rag:admin", "config:read", "config:write"]

[lightrag]
url = "http://localhost:9999"
timeout_seconds = 30
max_retries = 3
retry_delay_seconds = 1

[defaults]
query_mode = "hybrid"
top_k = 10
response_type = "simple"
"#;
    writeln!(temp_file, "{}", config_content).unwrap();
    let config_path = temp_file.path().to_str().unwrap().to_string();

    // 加载配置并构建 app
    std::env::set_var("CONFIG_PATH", &config_path);
    let config = Config::load().unwrap();
    let app = build_test_app(&config);

    // PATCH 请求修改 top_k
    let request = Request::builder()
        .method(Method::PATCH)
        .uri("/api/config")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"defaults":{"query_mode":"hybrid","top_k":20,"response_type":"simple"}}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 验证文件已更新
    let updated_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(updated_content.contains("top_k = 20"), "config file should be updated");

    std::env::remove_var("CONFIG_PATH");
}

#[tokio::test]
async fn test_api_config_patch_rejects_invalid_config() {
    let mut config = build_test_config("http://localhost:9999");
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("config:read".to_string());
        admin.scopes.push("config:write".to_string());
    }
    let app = build_test_app(&config);

    // 尝试设置无效的 top_k（超出范围）
    let request = Request::builder()
        .method(Method::PATCH)
        .uri("/api/config")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"defaults":{"query_mode":"hybrid","top_k":2000,"response_type":"simple"}}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// --- 迭代 5-7: Token 管理 API ---

// 迭代 5: GET /api/tokens (需要 token:read scope)

#[tokio::test]
async fn test_api_tokens_get_requires_auth() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/tokens")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_tokens_get_rejects_without_token_read_scope() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/tokens")
        .header(header::AUTHORIZATION, "Bearer alice-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_api_tokens_get_returns_masked_list() {
    let mut config = build_test_config("http://localhost:9999");
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("token:read".to_string());
    }
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/tokens")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let tokens = json["tokens"].as_array().expect("tokens array");
    assert!(tokens.len() >= 3); // alice, bob, admin

    // 验证 token 被脱敏为预览格式（前4后2）
    for token_obj in tokens {
        let preview = token_obj["token_preview"].as_str().expect("token_preview");
        assert!(preview.contains("..."), "token should be masked as preview");
        assert!(preview.len() < 20, "preview should be short");
    }
}

// 迭代 6: POST /api/tokens (需要 token:write scope)

#[tokio::test]
async fn test_api_tokens_post_requires_auth() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/tokens")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"name":"test","scopes":["rag:read"]}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_tokens_post_rejects_without_token_write_scope() {
    let mut config = build_test_config("http://localhost:9999");
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("token:read".to_string());
    }
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/tokens")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"name":"test","scopes":["rag:read"]}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_api_tokens_post_creates_token() {
    use tempfile::NamedTempFile;
    use std::io::Write;

    let mut temp_file = NamedTempFile::new().unwrap();
    let config_content = r#"
[server]
host = "127.0.0.1"
port = 8080

[mcp]
server_name = "test-server"
version = "1.0.0"

[auth]
audit_log_path = "./audit.log"

[[auth.tokens]]
name = "admin"
token = "admin-token"
scopes = ["rag:read", "rag:write", "rag:admin", "token:read", "token:write"]

[lightrag]
url = "http://localhost:9999"
timeout_seconds = 30
max_retries = 3
retry_delay_seconds = 1

[defaults]
query_mode = "hybrid"
top_k = 10
response_type = "simple"
"#;
    writeln!(temp_file, "{}", config_content).unwrap();
    let config_path = temp_file.path().to_str().unwrap().to_string();

    std::env::set_var("CONFIG_PATH", &config_path);
    let config = Config::load().unwrap();
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/tokens")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"name":"newuser","scopes":["rag:read"]}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // 返回完整 token（仅此一次）
    let token = json["token"].as_str().expect("token field");
    assert!(token.len() == 64, "token should be 64 chars (32 bytes hex)");
    assert_eq!(json["name"], "newuser");
    assert_eq!(json["scopes"], serde_json::json!(["rag:read"]));

    std::env::remove_var("CONFIG_PATH");
}

#[tokio::test]
async fn test_api_tokens_post_rejects_duplicate_name() {
    let mut config = build_test_config("http://localhost:9999");
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("token:write".to_string());
    }
    let app = build_test_app(&config);

    // 尝试创建已存在的 token name
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/tokens")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"name":"alice","scopes":["rag:read"]}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

// 迭代 7: DELETE /api/tokens/:name (需要 token:write scope)

#[tokio::test]
async fn test_api_tokens_delete_requires_auth() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/tokens/alice")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_tokens_delete_rejects_without_token_write_scope() {
    let mut config = build_test_config("http://localhost:9999");
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("token:read".to_string());
    }
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/tokens/alice")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_api_tokens_delete_removes_token() {
    use tempfile::NamedTempFile;
    use std::io::Write;

    let mut temp_file = NamedTempFile::new().unwrap();
    let config_content = r#"
[server]
host = "127.0.0.1"
port = 8080

[mcp]
server_name = "test-server"
version = "1.0.0"

[auth]
audit_log_path = "./audit.log"

[[auth.tokens]]
name = "admin"
token = "admin-token"
scopes = ["rag:read", "rag:write", "rag:admin", "token:write"]

[[auth.tokens]]
name = "victim"
token = "victim-token"
scopes = ["rag:read"]

[lightrag]
url = "http://localhost:9999"
timeout_seconds = 30
max_retries = 3
retry_delay_seconds = 1

[defaults]
query_mode = "hybrid"
top_k = 10
response_type = "simple"
"#;
    writeln!(temp_file, "{}", config_content).unwrap();
    let config_path = temp_file.path().to_str().unwrap().to_string();

    std::env::set_var("CONFIG_PATH", &config_path);
    let config = Config::load().unwrap();
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/tokens/victim")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");

    std::env::remove_var("CONFIG_PATH");
}

#[tokio::test]
async fn test_api_tokens_delete_returns_404_for_nonexistent() {
    let mut config = build_test_config("http://localhost:9999");
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("token:write".to_string());
    }
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/tokens/nonexistent")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// --- 迭代 8: GET /api/audit/logs (需要 audit:read scope) ---

#[tokio::test]
async fn test_api_audit_logs_requires_auth() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/audit/logs")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_audit_logs_rejects_without_audit_read_scope() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/audit/logs")
        .header(header::AUTHORIZATION, "Bearer alice-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_api_audit_logs_returns_empty_when_no_logs() {
    use tempfile::NamedTempFile;

    let temp_file = NamedTempFile::new().unwrap();
    let log_path = temp_file.path().to_str().unwrap().to_string();

    let mut config = build_test_config("http://localhost:9999");
    config.auth.audit_log_path = log_path;
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("audit:read".to_string());
    }
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/audit/logs")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["logs"].as_array().unwrap().len(), 0);
    assert_eq!(json["total"], 0);
}

#[tokio::test]
async fn test_api_audit_logs_parses_and_returns_entries() {
    use tempfile::NamedTempFile;
    use std::io::Write;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "[2026-05-04T10:00:00Z] user=alice tool=rag_query params=test result=success").unwrap();
    writeln!(temp_file, "[2026-05-04T10:01:00Z] user=bob tool=rag_insert params=data result=success").unwrap();
    writeln!(temp_file, "[2026-05-04T10:02:00Z] user=alice tool=rag_query params=another result=success").unwrap();
    temp_file.flush().unwrap();

    let log_path = temp_file.path().to_str().unwrap().to_string();

    let mut config = build_test_config("http://localhost:9999");
    config.auth.audit_log_path = log_path;
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("audit:read".to_string());
    }
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/audit/logs")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let logs = json["logs"].as_array().unwrap();
    assert_eq!(logs.len(), 3);
    assert_eq!(json["total"], 3);

    // 验证第一条日志
    assert_eq!(logs[0]["user"], "alice");
    assert_eq!(logs[0]["tool"], "rag_query");
    assert_eq!(logs[0]["params"], "test");
    assert_eq!(logs[0]["result"], "success");
}

#[tokio::test]
async fn test_api_audit_logs_supports_pagination() {
    use tempfile::NamedTempFile;
    use std::io::Write;

    let mut temp_file = NamedTempFile::new().unwrap();
    for i in 1..=10 {
        writeln!(temp_file, "[2026-05-04T10:00:{}Z] user=alice tool=rag_query params=test{} result=success", i, i).unwrap();
    }
    temp_file.flush().unwrap();

    let log_path = temp_file.path().to_str().unwrap().to_string();

    let mut config = build_test_config("http://localhost:9999");
    config.auth.audit_log_path = log_path;
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("audit:read".to_string());
    }
    let app = build_test_app(&config);

    // 请求第 2 页，每页 3 条
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/audit/logs?page=2&page_size=3")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let logs = json["logs"].as_array().unwrap();
    assert_eq!(logs.len(), 3); // 第 2 页的 3 条
    assert_eq!(json["total"], 10);
    assert_eq!(json["page"], 2);
    assert_eq!(json["page_size"], 3);

    // 验证是第 4-6 条（索引 3-5）
    assert_eq!(logs[0]["params"], "test4");
    assert_eq!(logs[1]["params"], "test5");
    assert_eq!(logs[2]["params"], "test6");
}

#[tokio::test]
async fn test_api_audit_logs_filters_by_user() {
    use tempfile::NamedTempFile;
    use std::io::Write;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "[2026-05-04T10:00:00Z] user=alice tool=rag_query params=test result=success").unwrap();
    writeln!(temp_file, "[2026-05-04T10:01:00Z] user=bob tool=rag_insert params=data result=success").unwrap();
    writeln!(temp_file, "[2026-05-04T10:02:00Z] user=alice tool=rag_query params=another result=success").unwrap();
    temp_file.flush().unwrap();

    let log_path = temp_file.path().to_str().unwrap().to_string();

    let mut config = build_test_config("http://localhost:9999");
    config.auth.audit_log_path = log_path;
    if let Some(admin) = config.auth.tokens.iter_mut().find(|t| t.name == "admin") {
        admin.scopes.push("audit:read".to_string());
    }
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/audit/logs?user=alice")
        .header(header::AUTHORIZATION, "Bearer admin-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let logs = json["logs"].as_array().unwrap();
    assert_eq!(logs.len(), 2);
    assert!(logs.iter().all(|log| log["user"] == "alice"));
}

// --- 迭代 9: 静态文件服务 ---

#[tokio::test]
async fn test_static_index_returns_html() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
    assert!(content_type.to_str().unwrap().contains("text/html"));

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("PangenMCP Admin"));
}

#[tokio::test]
async fn test_static_css_returns_stylesheet() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/style.css")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
    assert!(content_type.to_str().unwrap().contains("text/css"));
}

#[tokio::test]
async fn test_static_nonexistent_returns_404() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/nonexistent.js")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================
// 9. Metrics 端点测试
// ============================================================

#[tokio::test]
async fn test_metrics_endpoint_returns_prometheus_format() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    // 先记录一些指标
    pangenmcp::metrics::record_request("rag_query", "test", "success");
    pangenmcp::metrics::record_duration("rag_query", 100.0);
    pangenmcp::metrics::set_lightrag_status(true);
    pangenmcp::metrics::record_auth_failure("test_failure");

    let request = Request::builder()
        .method(Method::GET)
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();

    // 验证 Prometheus 格式（至少包含指标名称）
    assert!(text.contains("mcp_requests_total"));
    assert!(text.contains("mcp_request_duration_ms"));
    assert!(text.contains("lightrag_healthy"));
    assert!(text.contains("mcp_auth_failures_total"));
}

#[tokio::test]
async fn test_metrics_endpoint_accessible_without_auth() {
    let config = build_test_config("http://localhost:9999");
    let app = build_test_app(&config);

    // 不带 Authorization header
    let request = Request::builder()
        .method(Method::GET)
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // metrics 端点不需要认证
    assert_eq!(response.status(), StatusCode::OK);
}
