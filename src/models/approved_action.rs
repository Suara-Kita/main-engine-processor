use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentType {
    LocalIssue,
    PolicyAgenda,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingChannel {
    Whatsapp,
    Telegram,
    Facebook,
    FieldOps,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionMetadata {
    pub action_id: Uuid,
    pub approved_at: DateTime<Utc>,
    pub original_ingestion_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessedPayload {
    pub intent_type: String,
    pub scope: String,
    pub primary_category: String,
    pub sub_categories: Vec<String>,
    pub cleaned_summary: String,
    pub urgency: String,
    pub voter_sentiment: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingTarget {
    pub channel: RoutingChannel,
    pub client_identifier: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApprovedAction {
    pub action_metadata: ActionMetadata,
    pub processed_payload: ProcessedPayload,
    pub routing_target: RoutingTarget,
}
