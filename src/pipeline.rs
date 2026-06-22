use anyhow::Result;
use tracing::{error, info};

use crate::ai::llm::{LlmAnalysis, LlmClient};
use crate::db::neo4j::Neo4jClient;
use crate::db::pgvector::PgVectorClient;
use crate::db::postgres::PostgresClient;
use crate::models::voter_input::VoterInput;
use crate::queues::QueueClient;

pub struct Pipeline {
    llm: LlmClient,
    postgres: PostgresClient,
    pgvector: PgVectorClient,
    neo4j: Neo4jClient,
    queues: QueueClient,
}

impl Pipeline {
    pub fn new(
        llm: LlmClient,
        postgres: PostgresClient,
        pgvector: PgVectorClient,
        neo4j: Neo4jClient,
        queues: QueueClient,
    ) -> Self {
        Self { llm, postgres, pgvector, neo4j, queues }
    }

    pub async fn process(&self, input: VoterInput) -> Result<ProcessedMessage> {
        let ingestion_id = input.pipeline_metadata.ingestion_id;

        let ctx_text = input.context_anchor.as_ref().map(|c| c.parent_raw_text.clone());
        let known_categories = self.postgres.fetch_categories().await?;
        let force_fallback = input.pipeline_metadata.retry_count > 0;
        let analysis = self.llm.analyze(&input.content_payload.raw_text, &ctx_text, &known_categories, force_fallback).await?;

        if !analysis.has_substantive_value {
            info!(
                ingestion_id = %ingestion_id,
                reason = ?analysis.rejection_reason,
                "non-substantive message, skipping pipeline"
            );
            self.postgres.log_noise(&input, &analysis).await?;
            self.queues
                .increment_counter("stats:engine:messages_rejected")
                .await
                .ok();
            return Ok(ProcessedMessage {
                ingestion_id,
                summary: analysis.cleaned_summary,
                primary_category: analysis.primary_category,
                is_noise: true,
            });
        }

        let embedding = self
            .llm
            .generate_embedding(&analysis.cleaned_summary)
            .await
            .unwrap_or_default();

        let voter_profile_id = self.postgres.upsert_voter(&input).await?;
        self.postgres
            .insert_interaction(&input, voter_profile_id, &analysis)
            .await?;

        if !embedding.is_empty() {
            if let Err(e) = self.pgvector.store_embedding(voter_profile_id, &embedding).await {
                error!(%ingestion_id, error = %e, "failed to store embedding");
            }
        }

        if let Err(e) = self.link_graph(&input, &analysis).await {
            error!(%ingestion_id, error = %e, "neo4j step failed non-fatally");
        }

        self.queues
            .increment_counter("stats:engine:messages_processed")
            .await
            .ok();

        info!(
            ingestion_id = %ingestion_id,
            intent = %analysis.intent_type,
            scope = %analysis.scope,
            category = %analysis.primary_category,
            urgency = %analysis.urgency,
            sentiment = %analysis.voter_sentiment,
            "pipeline completed"
        );

        Ok(ProcessedMessage {
            ingestion_id,
            summary: analysis.cleaned_summary,
            primary_category: analysis.primary_category,
            is_noise: false,
        })
    }

    async fn link_graph(&self, input: &VoterInput, analysis: &LlmAnalysis) -> Result<()> {
        self.neo4j
            .upsert_voter(
                &input.source_profile.client_identifier,
                input.source_profile.display_name.as_deref(),
            )
            .await?;

        self.neo4j
            .upsert_issue(
                input.pipeline_metadata.ingestion_id,
                &analysis.cleaned_summary,
                &analysis.primary_category,
                &analysis.scope,
                &analysis.voter_sentiment,
                &analysis.inferred_location_tags,
            )
            .await?;

        self.neo4j
            .link_voter_to_issue(
                &input.source_profile.client_identifier,
                input.pipeline_metadata.ingestion_id,
            )
            .await?;

        let policies = self
            .neo4j
            .find_relevant_policies(&analysis.primary_category)
            .await?;

        for policy in &policies {
            self.neo4j
                .upsert_policy_node(policy, &analysis.primary_category)
                .await?;
            self.neo4j
                .link_issue_to_policy(input.pipeline_metadata.ingestion_id, policy)
                .await?;
        }

        Ok(())
    }
}

pub struct ProcessedMessage {
    pub ingestion_id: uuid::Uuid,
    pub summary: String,
    pub primary_category: String,
    pub is_noise: bool,
}
