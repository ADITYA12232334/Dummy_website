use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, sse::{Event, Sse}},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::{self, Stream};
use serde::Deserialize;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::sync::Arc;
use tokio::net::TcpListener;
use uuid::Uuid;
use crate::models::{ScanConfig, ScanResult};
use crate::redis_queue::{RedisQueue, JobPayload, JobOptions};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use std::convert::Infallible;
use std::time::Duration;

mod models;
mod redis_queue;

struct AppState {
    redis: RedisQueue,
    db: SqlitePool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Ensure reports directory exists
    tokio::fs::create_dir_all("reports").await?;

    // Setup Database
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:zap_scanner.db?mode=rwc".into());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Create tables if they don't exist
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS scan_configs (
            id BLOB PRIMARY KEY,
            name TEXT NOT NULL,
            urls TEXT NOT NULL,
            scan_type TEXT NOT NULL,
            spider_type TEXT NOT NULL,
            parse_robots BOOLEAN NOT NULL,
            parse_sitemap BOOLEAN NOT NULL,
            duration INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        )"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS scan_results (
            id BLOB PRIMARY KEY,
            job_id BLOB NOT NULL,
            config_name TEXT NOT NULL DEFAULT '',
            url TEXT NOT NULL,
            total_vulnerabilities INTEGER NOT NULL,
            high_sev INTEGER NOT NULL,
            medium_sev INTEGER NOT NULL,
            low_sev INTEGER NOT NULL,
            info_sev INTEGER NOT NULL,
            report_path TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )"
    ).execute(&pool).await?;

    // Setup Redis
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let redis_queue = RedisQueue::new(&redis_url)?;

    let state = Arc::new(AppState {
        redis: redis_queue,
        db: pool,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/configs", get(list_configs).post(create_config))
        .route("/api/configs/launch-all", post(launch_all))
        .route("/api/configs/{id}/launch", post(launch_config))
        .route("/api/results", get(list_results))
        .route("/api/jobs/{id}/results", post(post_job_result))
        .route("/api/jobs/{id}/events", get(get_job_events))
        .route("/api/jobs/{id}/status", get(get_job_status))
        .route("/api/jobs/active", get(list_active_jobs))
        .nest_service("/reports", ServeDir::new("reports"))
        .layer(cors)
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    println!("Backend listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn list_configs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let configs = sqlx::query_as::<_, ScanConfig>("SELECT * FROM scan_configs ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await;

    match configs {
        Ok(c) => Json(c).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateConfigInput {
    name: String,
    urls: Vec<String>,
    scan_type: String,
    duration: i32,
}

async fn create_config(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateConfigInput>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let urls_json = serde_json::to_string(&input.urls).unwrap();
    let created_at = chrono::Utc::now().timestamp();

    let result = sqlx::query(
        "INSERT INTO scan_configs (id, name, urls, scan_type, spider_type, parse_robots, parse_sitemap, duration, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(&input.name)
    .bind(&urls_json)
    .bind(&input.scan_type)
    .bind("spider")
    .bind(true)
    .bind(true)
    .bind(input.duration)
    .bind(created_at)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => {
            let config = ScanConfig {
                id,
                name: input.name,
                urls: urls_json,
                scan_type: input.scan_type,
                spider_type: "spider".into(),
                parse_robots: true,
                parse_sitemap: true,
                duration: input.duration,
                created_at,
            };
            (StatusCode::CREATED, Json(config)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn launch_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let config = sqlx::query_as::<_, ScanConfig>("SELECT * FROM scan_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await;

    match config {
        Ok(Some(c)) => {
            let targets: Vec<String> = serde_json::from_str(&c.urls).unwrap_or_default();
            let job_id = Uuid::new_v4();
            
            let payload = JobPayload {
                job_id: job_id.to_string(),
                config_name: c.name.clone(),
                targets,
                options: JobOptions {
                    max_duration: c.duration * 60,
                    scan_type: c.scan_type,
                },
            };

            if let Err(e) = state.redis.enqueue_job(&payload).await {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }

            (StatusCode::ACCEPTED, Json(payload)).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn launch_all(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let configs = sqlx::query_as::<_, ScanConfig>("SELECT * FROM scan_configs")
        .fetch_all(&state.db)
        .await;

    match configs {
        Ok(list) => {
            let mut launched = vec![];
            for c in list {
                let targets: Vec<String> = serde_json::from_str(&c.urls).unwrap_or_default();
                let job_id = Uuid::new_v4();
                let payload = JobPayload {
                    job_id: job_id.to_string(),
                    config_name: c.name.clone(),
                    targets,
                    options: JobOptions {
                        max_duration: c.duration * 60,
                        scan_type: c.scan_type,
                    },
                };
                if state.redis.enqueue_job(&payload).await.is_ok() {
                    launched.push(payload);
                }
            }
            (StatusCode::ACCEPTED, Json(launched)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_job_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.redis.get_job_status(&id).await {
        Ok(Some(status)) => Json(status).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_active_jobs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.redis.get_active_jobs().await {
        Ok(active) => Json(active).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_results(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let results = sqlx::query_as::<_, ScanResult>("SELECT * FROM scan_results ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await;

    match results {
        Ok(r) => Json(r).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct PostResultInput {
    config_name: String,
    url: String,
    total_vulnerabilities: i32,
    high_sev: i32,
    medium_sev: i32,
    low_sev: i32,
    info_sev: i32,
    report_path: String,
}

async fn post_job_result(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Json(input): Json<PostResultInput>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let created_at = chrono::Utc::now().timestamp();

    let result = sqlx::query(
        "INSERT INTO scan_results (id, job_id, config_name, url, total_vulnerabilities, high_sev, medium_sev, low_sev, info_sev, report_path, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(job_id)
    .bind(&input.config_name)
    .bind(&input.url)
    .bind(input.total_vulnerabilities)
    .bind(input.high_sev)
    .bind(input.medium_sev)
    .bind(input.low_sev)
    .bind(input.info_sev)
    .bind(&input.report_path)
    .bind(created_at)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => StatusCode::CREATED.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_job_events(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::unfold((state, id), |(state, id)| async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let status = state.redis.get_job_status(&id).await.ok().flatten();
        if let Some(s) = status {
            let event = Event::default().json_data(s).unwrap();
            Some((Ok(event), (state, id)))
        } else {
            None
        }
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}
