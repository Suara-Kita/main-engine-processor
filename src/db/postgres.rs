use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

use crate::ai::llm::LlmAnalysis;
use crate::models::voter_input::VoterInput;

pub struct PostgresClient {
    pool: PgPool,
}

impl PostgresClient {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn upsert_voter(&self, input: &VoterInput) -> Result<Uuid> {
        let row: (Uuid,) = sqlx::query_as(
            r#"
            INSERT INTO voter_profiles
                (client_identifier, source_channel, display_name, contact_info, inferred_constituency)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (client_identifier)
            DO UPDATE SET
                display_name = COALESCE($3, voter_profiles.display_name),
                contact_info = COALESCE($4, voter_profiles.contact_info),
                inferred_constituency = COALESCE($5, voter_profiles.inferred_constituency),
                last_interaction_at = NOW(),
                interaction_count = voter_profiles.interaction_count + 1
            RETURNING id
            "#,
        )
        .bind(&input.source_profile.client_identifier)
        .bind(input.pipeline_metadata.source_channel.as_str())
        .bind(&input.source_profile.display_name)
        .bind(&input.source_profile.contact_info)
        .bind(&input.source_profile.inferred_constituency)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    pub async fn insert_interaction(
        &self,
        input: &VoterInput,
        voter_profile_id: Uuid,
        analysis: &LlmAnalysis,
    ) -> Result<Uuid> {
        let status = "pending";

        let response_id: Option<String> = match input.pipeline_metadata.source_channel.as_str() {
            "telegram" => input.source_profile.client_identifier.strip_prefix("tg_").map(|s| s.to_string()),
            _ => None,
        };

        let row: (Uuid,) = sqlx::query_as(
            r#"
            INSERT INTO interactions
                (ingestion_id, voter_profile_id, source_channel, raw_text, content_type,
                 media_attachments, context_anchor,
                 intent_type, scope, primary_category, sub_categories,
                 cleaned_summary, urgency, voter_sentiment,
                 inferred_location_tags, rejection_reason, raw_language, status,
                 response_id, constituency,
                 ingested_at, processed_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, NOW())
            ON CONFLICT (ingestion_id) DO NOTHING
            RETURNING id
            "#,
        )
        .bind(input.pipeline_metadata.ingestion_id)
        .bind(voter_profile_id)
        .bind(input.pipeline_metadata.source_channel.as_str())
        .bind(&input.content_payload.raw_text)
        .bind(input.content_payload.content_type.as_str())
        .bind(&input.content_payload.media_attachments)
        .bind(serde_json::to_value(&input.context_anchor).ok())
        .bind(&analysis.intent_type)
        .bind(&analysis.scope)
        .bind(&analysis.primary_category)
        .bind(&analysis.sub_categories)
        .bind(&analysis.cleaned_summary)
        .bind(&analysis.urgency)
        .bind(&analysis.voter_sentiment)
        .bind(&analysis.inferred_location_tags)
        .bind(&analysis.rejection_reason)
        .bind(&analysis.detected_language)
        .bind(status)
        .bind(response_id)
        .bind(&input.source_profile.inferred_constituency)
        .bind(input.pipeline_metadata.ingested_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    pub async fn log_noise(&self, input: &VoterInput, voter_profile_id: Uuid, analysis: &LlmAnalysis) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO interactions
                (ingestion_id, voter_profile_id, source_channel, raw_text, content_type,
                 media_attachments, context_anchor,
                 intent_type, cleaned_summary, rejection_reason, raw_language, status,
                 constituency, ingested_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'noise', $12, $13)
            ON CONFLICT (ingestion_id) DO NOTHING
            "#,
        )
        .bind(input.pipeline_metadata.ingestion_id)
        .bind(voter_profile_id)
        .bind(input.pipeline_metadata.source_channel.as_str())
        .bind(&input.content_payload.raw_text)
        .bind(input.content_payload.content_type.as_str())
        .bind(&input.content_payload.media_attachments)
        .bind(serde_json::to_value(&input.context_anchor).ok())
        .bind(&analysis.intent_type)
        .bind(&analysis.cleaned_summary)
        .bind(&analysis.rejection_reason)
        .bind(&analysis.detected_language)
        .bind(&input.source_profile.inferred_constituency)
        .bind(input.pipeline_metadata.ingested_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_approved(&self, ingestion_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE interactions
            SET status = 'approved', approved_at = NOW()
            WHERE ingestion_id = $1
            "#,
        )
        .bind(ingestion_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_dispatched(&self, ingestion_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE interactions
            SET status = 'dispatched', dispatched_at = NOW()
            WHERE ingestion_id = $1
            "#,
        )
        .bind(ingestion_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_dispatch_error(&self, ingestion_id: Uuid, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE interactions
            SET status = 'dispatch_error', dispatch_error = $2
            WHERE ingestion_id = $1
            "#,
        )
        .bind(ingestion_id)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
