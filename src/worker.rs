use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{error, info};

use crate::config::Config;
use crate::pipeline::Pipeline;
use crate::queues::QueueClient;

const DLQ_QUEUE: &str = "queue:voter_inputs_dlq";

pub struct WorkerPool {
    pipelines: Arc<Vec<Pipeline>>,
    queues: QueueClient,
    config: Config,
    semaphore: Arc<Semaphore>,
}

impl WorkerPool {
    pub fn new(pipelines: Vec<Pipeline>, queues: QueueClient, config: Config) -> Self {
        let max_concurrent = config.worker_count;
        Self {
            pipelines: Arc::new(pipelines),
            queues,
            config,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub async fn run(&self) {
        let queue_name = self.config.queue_voter_inputs.clone();

        let snapshot_queues = self.queues.clone();
        let snapshot_queue = queue_name.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(30));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                let depth = snapshot_queues.queue_depth(&snapshot_queue).await;
                match depth {
                    Ok(d) => info!(queue = %snapshot_queue, depth = %d, "queue snapshot"),
                    Err(e) => error!(error = %e, "failed to query queue depth"),
                }
            }
        });

        info!(
            queue = %queue_name,
            workers = %self.config.worker_count,
            "worker pool started"
        );

        loop {
            let result = self.queues.pop::<crate::models::voter_input::VoterInput>(&queue_name).await;
            match result {
                Ok(Some(input)) => {
                    let text_preview: String = input.content_payload.raw_text.chars().take(120).collect();
                    info!(
                        queue = %queue_name,
                        ingestion_id = %input.pipeline_metadata.ingestion_id,
                        preview = %text_preview,
                        "worker popped message from queue"
                    );

                    let permit = self.semaphore.clone().acquire_owned().await;
                    let pipelines = self.pipelines.clone();
                    let queues_for_dlq = self.queues.clone();
                    let raw_json = serde_json::to_string(&input).unwrap_or_default();
                    let ingestion_id = input.pipeline_metadata.ingestion_id;

                    let pipeline_idx = {
                        let bytes = ingestion_id.to_string();
                        let hash = bytes.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
                        (hash % pipelines.len() as u64) as usize
                    };

                    tokio::spawn(async move {
                        let _permit = permit;
                        let pipeline = &pipelines[pipeline_idx];
                        if let Err(e) = pipeline.process(input).await {
                            error!(error = %e, ingestion_id = %ingestion_id, "pipeline processing failed");
                            let _ = queues_for_dlq.push_raw(DLQ_QUEUE, &raw_json).await;
                        }
                    });
                }
                Ok(None) => {}
                Err(e) => {
                    error!(error = %e, "BRPOP failed, reconnecting in 5s");
                    self.queues.sleep(5).await;
                }
            }
        }
    }
}
