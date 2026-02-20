use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ScanConfig {
    pub id: Uuid,
    pub name: String,
    pub urls: String, // JSON array of strings
    pub scan_type: String,
    pub spider_type: String,
    pub parse_robots: bool,
    pub parse_sitemap: bool,
    pub duration: i32,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ScanResult {
    pub id: Uuid,
    pub job_id: Uuid,
    pub config_name: String,
    pub url: String,
    pub total_vulnerabilities: i32,
    pub high_sev: i32,
    pub medium_sev: i32,
    pub low_sev: i32,
    pub info_sev: i32,
    pub report_path: String,
    pub created_at: i64,
}
