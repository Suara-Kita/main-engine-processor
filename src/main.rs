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

    let mut pipelines = Vec::with_capacity(cfg.worker_count);
    for _ in 0..cfg.worker_count {
        let llm = ai::llm::LlmClient::new(&cfg.llm_endpoint, &cfg.llm_model, &cfg.llm_api_key);
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
