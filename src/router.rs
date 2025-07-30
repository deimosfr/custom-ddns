use axum::{
    Router,
    response::{Html, Json},
    routing::get,
};
use serde_json::json;
use tracing::info;

pub async fn start_health_server(port: u16) -> Result<(), anyhow::Error> {
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_check));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("Health check server listening on port {}", port);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn root_handler() -> Html<String> {
    let html = format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Custom DDNS Service</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            max-width: 800px;
            margin: 0 auto;
            padding: 2rem;
            background-color: #f8f9fa;
            color: #333;
        }}
        .container {{
            background: white;
            border-radius: 8px;
            padding: 2rem;
            box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
        }}
        h1 {{
            color: #2c3e50;
            margin-bottom: 0.5rem;
        }}
        .version {{
            color: #7f8c8d;
            font-size: 0.9rem;
            margin-bottom: 2rem;
        }}
        .endpoint {{
            background: #f8f9fa;
            border: 1px solid #e9ecef;
            border-radius: 6px;
            padding: 1rem;
            margin-bottom: 1rem;
        }}
        .endpoint h3 {{
            margin: 0 0 0.5rem 0;
            color: #495057;
        }}
        .method {{
            display: inline-block;
            background: #007bff;
            color: white;
            padding: 0.2rem 0.5rem;
            border-radius: 4px;
            font-size: 0.8rem;
            font-weight: bold;
            margin-right: 0.5rem;
        }}
        .description {{
            color: #6c757d;
            margin: 0.5rem 0;
        }}
        a {{
            color: #007bff;
            text-decoration: none;
        }}
        a:hover {{
            text-decoration: underline;
        }}
        .status {{
            display: inline-block;
            background: #28a745;
            color: white;
            padding: 0.3rem 0.8rem;
            border-radius: 20px;
            font-size: 0.8rem;
            font-weight: bold;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🌐 Custom DDNS Service</h1>
        <div class="version">Version {}</div>
        <div class="status">🟢 Service Running</div>
        
        <h2>Available Endpoints</h2>
        
        <div class="endpoint">
            <h3><span class="method">GET</span><a href="/">/</a></h3>
            <div class="description">Service information and available endpoints</div>
        </div>
        
        <div class="endpoint">
            <h3><span class="method">GET</span><a href="/health">/health</a></h3>
            <div class="description">Health check endpoint for Kubernetes liveness and readiness probes</div>
        </div>
        
        <h2>About</h2>
        <p>This is a custom Dynamic DNS (DDNS) service that automatically updates DNS records when your IP address changes. The service monitors your IP address and updates DNS providers like Cloudflare when changes are detected.</p>
        
        <h2>Health Monitoring</h2>
        <p>Kubernetes health probes are configured to check the <a href="/health">/health</a> endpoint to ensure the service is running properly.</p>
    </div>
</body>
</html>
"#,
        env!("CARGO_PKG_VERSION")
    );

    Html(html)
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "service": "custom-ddns",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}
