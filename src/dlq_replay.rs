use std::time::Duration;

use tracing::{error, info, warn};

use crate::models::voter_input::VoterInput;
use crate::queues::QueueClient;

const DLQ_QUEUE: &str = "queue:voter_inputs_dlq";
const MAIN_QUEUE: &str = "queue:voter_inputs";
const REPLAY_INTERVAL_SECS: u64 = 600;
const MAX_RETRIES: u32 = 5;

pub async fn run(queues: QueueClient) {
    info!("DLQ replay task started (interval: {}s)", REPLAY_INTERVAL_SECS);

    drain_dlq(&queues).await;

    loop {
        tokio::time::sleep(Duration::from_secs(REPLAY_INTERVAL_SECS)).await;
        drain_dlq(&queues).await;
    }
}

async fn drain_dlq(queues: &QueueClient) {
    info!("DLQ replay cycle starting");
    loop {
        match queues.rpop::<VoterInput>(DLQ_QUEUE).await {
            Ok(Some(mut msg)) => {
                msg.pipeline_metadata.retry_count += 1;
                if msg.pipeline_metadata.retry_count > MAX_RETRIES {
                    warn!(
                        ingestion_id = %msg.pipeline_metadata.ingestion_id,
                        retry_count = %msg.pipeline_metadata.retry_count,
                        "message exceeded max retries, discarding"
                    );
                    continue;
                }
                info!(
                    ingestion_id = %msg.pipeline_metadata.ingestion_id,
                    retry_count = %msg.pipeline_metadata.retry_count,
                    "replaying message from DLQ"
                );
                if let Err(e) = queues.push(MAIN_QUEUE, &msg).await {
                    error!(
                        error = %e,
                        ingestion_id = %msg.pipeline_metadata.ingestion_id,
                        "failed to push replayed message back to main queue"
                    );
                }
            }
            Ok(None) => break,
            Err(e) => {
                error!(error = %e, "RPOP DLQ failed");
                break;
            }
        }
    }
    info!("DLQ replay cycle complete");
}
