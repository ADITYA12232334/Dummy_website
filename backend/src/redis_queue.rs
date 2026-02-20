use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct JobPayload {
    pub job_id: String,
    pub config_name: String,
    pub targets: Vec<String>,
    pub options: JobOptions,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobOptions {
    pub max_duration: i32,
    pub scan_type: String,
}

pub struct RedisQueue {
    client: Client,
}

impl RedisQueue {
    pub fn new(url: &str) -> anyhow::Result<Self> {
        let client = Client::open(url)?;
        Ok(Self { client })
    }

    pub async fn enqueue_job(&self, payload: &JobPayload) -> anyhow::Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let json = serde_json::to_string(payload)?;
        
        // Push to job queue and store details
        let _: () = redis::pipe()
            .atomic()
            .hset("scan:job_details", &payload.job_id, &json)
            .lpush("scan:jobs", &json)
            .query_async(&mut conn)
            .await?;
            
        Ok(())
    }

    pub async fn get_job_status(&self, job_id: &str) -> anyhow::Result<Option<std::collections::HashMap<String, String>>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("scan:status:{}", job_id);
        let status: std::collections::HashMap<String, String> = conn.hgetall(key).await?;
        
        if status.is_empty() {
            Ok(None)
        } else {
            Ok(Some(status))
        }
    }

    pub async fn get_active_jobs(&self) -> anyhow::Result<Vec<std::collections::HashMap<String, String>>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let inflight: std::collections::HashMap<String, String> = conn.hgetall("scan:inflight").await?;
        
        let mut active = vec![];
        for job_id in inflight.keys() {
            if let Some(mut status) = self.get_job_status(job_id).await? {
                status.insert("job_id".to_string(), job_id.clone());
                active.push(status);
            }
        }
        Ok(active)
    }
}
