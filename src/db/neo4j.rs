use anyhow::Result;
use neo4rs::{Graph, Node, Query};
use uuid::Uuid;

pub struct Neo4jClient {
    graph: Graph,
}

impl Neo4jClient {
    pub async fn new(uri: &str, user: &str, password: &str) -> Result<Self> {
        let config = neo4rs::ConfigBuilder::default()
            .uri(uri)
            .user(user)
            .password(password)
            .build()?;
        let graph = Graph::connect(config).await?;
        Ok(Self { graph })
    }

    pub async fn upsert_voter(&self, client_identifier: &str, display_name: Option<&str>) -> Result<Node> {
        let mut result = self
            .graph
            .execute(
                Query::new(
                    r#"
                    MERGE (v:Voter {client_identifier: $client_identifier})
                    ON CREATE SET v.display_name = $display_name, v.created_at = timestamp()
                    ON MATCH SET v.last_seen_at = timestamp()
                    RETURN v
                    "#
                    .to_string(),
                )
                .param("client_identifier", client_identifier)
                .param("display_name", display_name),
            )
            .await?;

        result
            .next()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Neo4j MERGE returned no node"))
            .map(|row| row.get("v").unwrap())
    }

    pub async fn upsert_issue(
        &self,
        interaction_id: Uuid,
        summary: &str,
        category: &str,
        scope: &str,
        sentiment: &str,
        location_tags: &[String],
    ) -> Result<Node> {
        let mut result = self
            .graph
            .execute(
                Query::new(
                    r#"
                    MERGE (i:Issue {interaction_id: $interaction_id})
                    ON CREATE SET
                        i.summary = $summary,
                        i.category = $category,
                        i.scope = $scope,
                        i.sentiment = $sentiment,
                        i.location_tags = $location_tags,
                        i.created_at = timestamp()
                    ON MATCH SET
                        i.summary = $summary,
                        i.category = $category,
                        i.scope = $scope,
                        i.sentiment = $sentiment,
                        i.location_tags = $location_tags
                    RETURN i
                    "#
                    .to_string(),
                )
                .param("interaction_id", interaction_id.to_string())
                .param("summary", summary)
                .param("category", category)
                .param("scope", scope)
                .param("sentiment", sentiment)
                .param("location_tags", location_tags),
            )
            .await?;

        result
            .next()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Neo4j MERGE returned no node"))
            .map(|row| row.get("i").unwrap())
    }

    pub async fn upsert_policy_node(&self, name: &str, category: &str) -> Result<Node> {
        let mut result = self
            .graph
            .execute(
                Query::new(
                    r#"
                    MERGE (p:Policy {name: $name})
                    ON CREATE SET p.category = $category, p.created_at = timestamp()
                    RETURN p
                    "#
                    .to_string(),
                )
                .param("name", name)
                .param("category", category),
            )
            .await?;

        result
            .next()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Neo4j MERGE returned no node"))
            .map(|row| row.get("p").unwrap())
    }

    pub async fn link_voter_to_issue(&self, voter_id: &str, interaction_id: Uuid) -> Result<()> {
        self.graph
            .run(
                Query::new(
                    r#"
                    MATCH (v:Voter {client_identifier: $voter_id})
                    MATCH (i:Issue {interaction_id: $interaction_id})
                    MERGE (v)-[r:RAISED]->(i)
                    ON CREATE SET r.raised_at = timestamp()
                    "#
                    .to_string(),
                )
                .param("voter_id", voter_id)
                .param("interaction_id", interaction_id.to_string()),
            )
            .await?;
        Ok(())
    }

    pub async fn link_issue_to_policy(&self, interaction_id: Uuid, policy_name: &str) -> Result<()> {
        self.graph
            .run(
                Query::new(
                    r#"
                    MATCH (i:Issue {interaction_id: $interaction_id})
                    MATCH (p:Policy {name: $policy_name})
                    MERGE (i)-[r:MAPS_TO]->(p)
                    ON CREATE SET r.linked_at = timestamp()
                    "#
                    .to_string(),
                )
                .param("interaction_id", interaction_id.to_string())
                .param("policy_name", policy_name),
            )
            .await?;
        Ok(())
    }

    pub async fn find_relevant_policies(&self, category: &str) -> Result<Vec<String>> {
        let mut result = self
            .graph
            .execute(
                Query::new(
                    r#"
                    MATCH (p:Policy {category: $category})
                    RETURN p.name AS name
                    LIMIT 10
                    "#
                    .to_string(),
                )
                .param("category", category),
            )
            .await?;

        let mut names = Vec::new();
        while let Some(row) = result.next().await? {
            names.push(row.get::<String>("name").unwrap());
        }
        Ok(names)
    }
}
