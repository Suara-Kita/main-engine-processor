mod ai;
mod config;
mod db;
mod models;
mod pipeline;
mod queues;
mod worker;

#[cfg(test)]
mod test_helpers;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cfg = config::Config::from_env()?;

    let queues = queues::QueueClient::new(&cfg.redis_url).await?;

    // Background task: consume queue:dispatched and update interaction status
    {
        let dispatcher_queues = queues.clone();
        let dispatcher_pg = db::postgres::PostgresClient::new(&cfg.database_url).await?;
        let dispatcher_queue_name = cfg.queue_dispatched.clone();

        tokio::spawn(async move {
            use tracing::{error, info};
            info!(queue = %dispatcher_queue_name, "dispatch status consumer started");
            loop {
                let result = dispatcher_queues.pop::<serde_json::Value>(&dispatcher_queue_name).await;
                match result {
                    Ok(Some(payload)) => {
                        let ingestion_id = payload
                            .get("ingestion_id")
                            .and_then(|v| v.as_str())
                            .and_then(|s| uuid::Uuid::parse_str(s).ok());
                        let status = payload.get("status").and_then(|v| v.as_str());

                        match (ingestion_id, status) {
                            (Some(id), Some("success")) => {
                                if let Err(e) = dispatcher_pg.mark_dispatched(id).await {
                                    error!(error = %e, ingestion_id = %id, "failed to mark dispatched");
                                }
                            }
                            (Some(id), Some("error")) => {
                                let error_msg = payload
                                    .get("error")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                if let Err(e) = dispatcher_pg.mark_dispatch_error(id, error_msg).await {
                                    error!(error = %e, ingestion_id = %id, "failed to mark dispatch error");
                                }
                            }
                            _ => {
                                error!(payload = %payload, "invalid dispatched payload");
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        error!(error = %e, "BRPOP dispatched_queue failed, reconnecting in 5s");
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });
    }

    let mut pipelines = Vec::with_capacity(cfg.worker_count);
    for _ in 0..cfg.worker_count {
        let llm = ai::llm::LlmClient::new(
            &cfg.llm_endpoint,
            &cfg.llm_model,
            &cfg.llm_api_key,
            &cfg.fallback_llm_endpoint,
            &cfg.fallback_llm_model,
            &cfg.fallback_llm_api_key,
        );
        let pg = db::postgres::PostgresClient::new(&cfg.database_url).await?;
        let pv = db::pgvector::PgVectorClient::new(pg.pool().clone());
        let n4j = db::neo4j::Neo4jClient::new(&cfg.neo4j_uri, &cfg.neo4j_user, &cfg.neo4j_password).await?;
        let q = queues::QueueClient::new(&cfg.redis_url).await?;

        pipelines.push(pipeline::Pipeline::new(llm, pg, pv, n4j, q, cfg.clone()));
    }

    let pool = worker::WorkerPool::new(pipelines, queues, cfg);
    pool.run().await;

    Ok(())
}
