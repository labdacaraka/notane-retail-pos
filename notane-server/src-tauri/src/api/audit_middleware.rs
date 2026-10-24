use axum::{
    extract::{Request, State},
    http::Method,
    middleware::Next,
    response::Response,
};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn audit_log_middleware(
    State(pool): State<PgPool>,
    req: Request,
    next: Next,
) -> Response {
    // 1. Lewati request OPTIONS (CORS preflight)
    if req.method() == Method::OPTIONS {
        return next.run(req).await;
    }

    let uri = req.uri().clone();
    let path = uri.path().to_string();

    // 2. Lewati endpoint non-audit, WebSocket, assets upload, atau recursive audit calls
    if path.contains("/auth/audit-logs")
        || path.starts_with("/ws/")
        || path.starts_with("/uploads/")
        || path.ends_with("/ping")
    {
        return next.run(req).await;
    }

    let method = req.method().to_string();
    let query = uri.query().map(|q| q.to_string());
    let headers = req.headers().clone();

    // 3. Ekstrak identitas user dari JWT Bearer
    let mut user_id: Option<Uuid> = None;
    let mut user_name: Option<String> = None;
    let mut role_name: Option<String> = None;
    let mut terminal_id: Option<String> = None;

    if let Some(auth_val) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_val.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if let Ok(claims) = crate::utils::jwt::verify_jwt(token.trim()) {
                    user_id = Some(claims.sub);
                    user_name = Some(claims.username);
                    role_name = Some(claims.role_name);
                    terminal_id = claims.terminal_id;
                }
            }
        }
    }

    // Terminal ID dari header jika tidak ada di JWT
    if terminal_id.is_none() {
        if let Some(t) = headers.get("X-Terminal-Id").or_else(|| headers.get("X-Device-Name")) {
            if let Ok(t_str) = t.to_str() {
                let clean = t_str.trim();
                if !clean.is_empty() {
                    terminal_id = Some(clean.to_string());
                }
            }
        }
    }
    let terminal_final = terminal_id.unwrap_or_else(|| "CLIENT".to_string());

    // IP Address
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|val| val.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .or_else(|| Some("127.0.0.1".to_string()));

    // User Agent
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|val| val.to_str().ok())
        .map(|s| s.to_string());

    // 4. Ekstrak Modul / Entitas & Action dari URL Path
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut entity_name = "system".to_string();
    let mut entity_id: Option<String> = None;

    if !segments.is_empty() {
        let seg_start = if segments[0] == "api" && segments.len() > 2 && segments[1] == "v1" {
            2
        } else {
            0
        };
        if seg_start < segments.len() {
            entity_name = segments[seg_start].to_string();
            let last = segments[segments.len() - 1];
            if Uuid::parse_str(last).is_ok() || last.chars().all(|c| c.is_ascii_digit()) {
                entity_id = Some(last.to_string());
            }
        }
    }

    let public_path = if path.starts_with("/api/v1") {
        path.clone()
    } else {
        format!("/api/v1{}", path)
    };
    let action = format!("{} {}", method, public_path);

    // 5. Intersepsi Body Payload tanpa merusak streaming downstream
    let (parts, body) = req.into_parts();
    let max_bytes = 1024 * 1024; // Maksimal 1MB dibaca untuk log
    let bytes = match axum::body::to_bytes(body, max_bytes).await {
        Ok(b) => b,
        Err(_) => axum::body::Bytes::new(),
    };

    let reconstructed_req = Request::from_parts(parts, axum::body::Body::from(bytes.clone()));

    // Sensor & Sanitasi Payload
    let mut sanitized_payload: Option<serde_json::Value> = None;
    if !bytes.is_empty() {
        if let Ok(mut json_val) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            sanitize_json(&mut json_val);
            if user_name.is_none() && path.contains("/auth/login") {
                if let Some(u) = json_val.get("username").and_then(|v| v.as_str()) {
                    user_name = Some(u.to_string());
                }
            }
            sanitized_payload = Some(json_val);
        } else if bytes.len() <= 2000 {
            if let Ok(text) = std::str::from_utf8(&bytes) {
                sanitized_payload = Some(serde_json::json!({ "_raw": text }));
            }
        } else {
            sanitized_payload = Some(serde_json::json!({ "_binary_upload": true, "size_bytes": bytes.len() }));
        }
    } else if let Some(ref q) = query {
        sanitized_payload = Some(serde_json::json!({ "_query": q }));
    }

    // 6. Jalankan Handler API Asli & Catat Latensi
    let start_time = std::time::Instant::now();
    let response = next.run(reconstructed_req).await;
    let latency_ms = start_time.elapsed().as_millis() as i32;
    let status_code = response.status().as_u16() as i32;

    // 7. Simpan Log Secara Asinkron (Non-Blocking di Background Task)
    let pool_clone = pool.clone();
    let endpoint_full = if let Some(ref q) = query {
        format!("{}?{}", path, q)
    } else {
        path
    };

    tokio::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO audit_logs (
                user_id, terminal_id, action, entity_name, entity_id,
                new_values, ip_address, http_method, endpoint, status_code,
                execution_time_ms, user_agent, user_name, role_name,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10,
                $11, $12, $13, $14,
                CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
            )",
        )
        .bind(user_id)
        .bind(terminal_final)
        .bind(action)
        .bind(entity_name)
        .bind(entity_id)
        .bind(sanitized_payload)
        .bind(ip_address)
        .bind(method)
        .bind(endpoint_full)
        .bind(status_code)
        .bind(latency_ms)
        .bind(user_agent)
        .bind(user_name)
        .bind(role_name)
        .execute(&pool_clone)
        .await;
    });

    response
}

fn sanitize_json(val: &mut serde_json::Value) {
    match val {
        serde_json::Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                let k_lower = k.to_lowercase();
                if k_lower.contains("password")
                    || k_lower == "pin"
                    || k_lower.contains("secret")
                    || k_lower.contains("token")
                    || k_lower == "pairing_code"
                {
                    *v = serde_json::json!("***");
                } else {
                    sanitize_json(v);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                sanitize_json(item);
            }
        }
        _ => {}
    }
}
