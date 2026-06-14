use anyhow::Result;
use sqlx::PgPool;

pub struct PgVectorClient {
    pool: PgPool,
}

impl PgVectorClient {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn store_embedding(&self, interaction_id: uuid::Uuid, embedding: &[f32]) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO issue_embeddings (interaction_id, embedding)
            VALUES ($1, $2::vector)
            "#,
        )
        .bind(interaction_id)
        .bind(embedding.to_vec())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_similar(&self, embedding: &[f32], limit: i64) -> Result<Vec<(uuid::Uuid, f64)>> {
        let rows: Vec<(uuid::Uuid, f64)> = sqlx::query_as(
            r#"
            SELECT ie.interaction_id, 1 - (ie.embedding <=> $1::vector) AS similarity
            FROM issue_embeddings ie
            ORDER BY ie.embedding <=> $1::vector
            LIMIT $2
            "#,
        )
        .bind(embedding.to_vec())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
