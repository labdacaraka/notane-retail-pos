use std::path::Path;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::ws::AppState;
use crate::models::export::{CreateExportJobRequest, ExportJobListQuery, ExportJob};
use crate::services::export_service::{create_export_job, spawn_export_worker};
use crate::utils::jwt::extract_claims;

pub fn routes(pool: PgPool, ws_state: AppState) -> Router {
    Router::new()
        .route("/exports", post(create_export_handler).get(list_exports_handler))
        .route("/exports/{id}/status", get(get_export_status_handler))
        .route("/exports/{id}/download", get(download_export_handler))
        .with_state((pool, ws_state))
}

async fn create_export_handler(
    State((pool, ws_state)): State<(PgPool, AppState)>,
    headers: HeaderMap,
    Json(payload): Json<CreateExportJobRequest>,
) -> impl IntoResponse {
    let user_id = extract_claims(&headers).ok().map(|c| c.sub);
    let outlet_id = crate::utils::jwt::resolve_effective_outlet_id(&pool, &headers, None).await;

    let filter = payload.filter_params.unwrap_or_else(|| json!({}));

    match create_export_job(
        &pool,
        user_id,
        outlet_id,
        &payload.module,
        &payload.title,
        filter,
    )
    .await
    {
        Ok(job) => {
            // Jalankan worker secara asinkron di latar belakang
            spawn_export_worker(pool.clone(), job.id, Some(ws_state.tx.clone()));

            (
                StatusCode::ACCEPTED,
                Json(json!({
                    "success": true,
                    "message": "Pekerjaan ekspor berhasil dijadwalkan di latar belakang.",
                    "data": job
                })),
            )
                .into_response()
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": err
            })),
        )
            .into_response(),
    }
}

async fn list_exports_handler(
    State((pool, _)): State<(PgPool, AppState)>,
    headers: HeaderMap,
    Query(query): Query<ExportJobListQuery>,
) -> impl IntoResponse {
    let current_user_id = extract_claims(&headers).ok().map(|c| c.sub);

    let mut sql = "SELECT * FROM export_jobs WHERE 1=1".to_string();

    if let Some(uid) = current_user_id {
        sql.push_str(&format!(" AND (user_id = '{}' OR user_id IS NULL)", uid));
    }
    if let Some(ref m) = query.module {
        sql.push_str(&format!(" AND module = '{}'", m.replace('\'', "''")));
    }
    if let Some(ref s) = query.status {
        sql.push_str(&format!(" AND status = '{}'", s.replace('\'', "''")));
    }

    sql.push_str(" ORDER BY created_at DESC LIMIT 50");

    match sqlx::query_as::<_, ExportJob>(&sql).fetch_all(&pool).await {
        Ok(list) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": list
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": err.to_string()
            })),
        )
            .into_response(),
    }
}

async fn get_export_status_handler(
    State((pool, _)): State<(PgPool, AppState)>,
    AxumPath(id): AxumPath<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, ExportJob>("SELECT * FROM export_jobs WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await
    {
        Ok(Some(job)) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": job
            })),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "message": "Pekerjaan ekspor tidak ditemukan."
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": err.to_string()
            })),
        )
            .into_response(),
    }
}

async fn download_export_handler(
    State((pool, _)): State<(PgPool, AppState)>,
    AxumPath(id): AxumPath<Uuid>,
) -> impl IntoResponse {
    let job = match sqlx::query_as::<_, ExportJob>("SELECT * FROM export_jobs WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await
    {
        Ok(Some(j)) => j,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "message": "Pekerjaan ekspor tidak ditemukan." })),
            )
                .into_response();
        }
    };

    if job.status != "COMPLETED" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "File ekspor belum selesai diproses." })),
        )
            .into_response();
    }

    let file_path = match job.file_path {
        Some(p) => p,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "message": "Lokasi file ekspor tidak tercatat." })),
            )
                .into_response();
        }
    };

    if !Path::new(&file_path).exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "message": "File fisik hasil ekspor tidak ditemukan pada disk server." })),
        )
            .into_response();
    }

    let bytes = match tokio::fs::read(&file_path).await {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": format!("Gagal membaca file: {}", e) })),
            )
                .into_response();
        }
    };

    let body = axum::body::Body::from(bytes);

    let content_disposition = format!("attachment; filename=\"{}\"", job.filename);

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::CONTENT_TYPE,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            .parse()
            .unwrap(),
    );
    response_headers.insert(
        header::CONTENT_DISPOSITION,
        content_disposition.parse().unwrap(),
    );

    (StatusCode::OK, response_headers, body).into_response()
}
