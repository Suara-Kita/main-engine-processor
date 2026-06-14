use serde::Serialize;
use tracing::{error, info, warn};

use serde::Deserialize;

#[derive(Debug, Clone, Serialize)]
pub struct LlmAnalysis {
    pub has_substantive_value: bool,
    pub rejection_reason: Option<String>,
    pub intent_type: String,
    pub scope: String,
    pub cleaned_summary: String,
    pub primary_category: String,
    pub sub_categories: Vec<String>,
    pub urgency: String,
    pub voter_sentiment: String,
    pub inferred_location_tags: Vec<String>,
    pub detected_language: String,
}

const REQUIRED_FIELDS: &[&str] = &[
    "has_substantive_value",
    "intent_type",
    "scope",
    "cleaned_summary",
    "primary_category",
    "sub_categories",
    "urgency",
    "voter_sentiment",
    "inferred_location_tags",
    "detected_language",
];

impl LlmAnalysis {
    fn from_json_value(v: &serde_json::Value) -> Self {
        Self {
            has_substantive_value: v
                .get("has_substantive_value")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            rejection_reason: v
                .get("rejection_reason")
                .and_then(|v| v.as_str())
                .map(String::from),
            intent_type: v
                .get("intent_type")
                .and_then(|v| v.as_str())
                .unwrap_or("noise")
                .to_string(),
            scope: v
                .get("scope")
                .and_then(|v| v.as_str())
                .unwrap_or("local")
                .to_string(),
            cleaned_summary: v
                .get("cleaned_summary")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            primary_category: v
                .get("primary_category")
                .and_then(|v| v.as_str())
                .unwrap_or("other")
                .to_string(),
            sub_categories: v
                .get("sub_categories")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            urgency: v
                .get("urgency")
                .and_then(|v| v.as_str())
                .unwrap_or("low")
                .to_string(),
            voter_sentiment: v
                .get("voter_sentiment")
                .and_then(|v| v.as_str())
                .unwrap_or("neutral")
                .to_string(),
            inferred_location_tags: v
                .get("inferred_location_tags")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            detected_language: v
                .get("detected_language")
                .and_then(|v| v.as_str())
                .unwrap_or("other")
                .to_string(),
        }
    }
}

fn missing_required_fields(v: &serde_json::Value) -> Vec<String> {
    let mut missing = Vec::new();
    for field in REQUIRED_FIELDS {
        let present = match v.get(field) {
            Some(val) => !val.is_null(),
            None => false,
        };
        if !present {
            missing.push(field.to_string());
        }
    }
    missing
}

#[derive(Debug, Clone, Serialize)]
struct LlmRequest {
    model: String,
    messages: Vec<Message>,
    response_format: ResponseFormat,
}

#[derive(Debug, Clone, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
    json_schema: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct LlmResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Clone, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Clone, Deserialize)]
struct ChoiceMessage {
    content: String,
}

pub struct LlmClient {
    client: reqwest::Client,
    endpoint: String,
    model: String,
    api_key: String,
}

const SYSTEM_PROMPT: &str = r#"You are a political issue analyzer for a Malaysian constituency management system. Analyze the voter's message and return a JSON object with the following fields:

1. has_substantive_value (boolean) — Is this message worth showing to humans? False for pure greetings ("Hi, apa khabar"), spam, advertisements, pure insults or personal attacks without a specific identifiable real-world issue (e.g., "Bodoh la UMNO, mati je" with no concrete complaint), or fully non-actionable content. True for any specific issue, complaint, request, support/opposition statement, or policy opinion.

2. rejection_reason (string or null) — If has_substantive_value is false, briefly explain why. Otherwise null.

3. intent_type (enum: "local_issue" | "policy_agenda" | "noise") — "local_issue" for localized solvable problems (pothole, broken drain). "policy_agenda" for macroeconomic or ideological ideas (minimum wage, education reform). "noise" only when has_substantive_value is false.

4. scope (enum: "local" | "state" | "national") — The impact level of the issue.

5. cleaned_summary (string) — A clean 1-sentence objective summary. Fix typos, expand slang, normalize dialect. Write in formal Bahasa Melayu. This is what humans read first.

6. primary_category (enum: "infrastructure" | "economy_and_labor" | "welfare_and_aid" | "education" | "healthcare" | "religion_and_community") — The primary pillar this issue falls under.

7. sub_categories (array of strings) — 1-3 granular tags for filtering, e.g., ["potholes", "road_safety"] or ["tolls", "public_transport"].

8. urgency (enum: "low" | "medium" | "high") — How critical. Floods, accidents, safety hazards → high.

9. voter_sentiment (enum: "supportive" | "frustrated" | "neutral" | "demanding") — The emotional tone.

10. inferred_location_tags (array of strings) — Any location mentions (neighborhood, town, constituency, landmark). Empty array if none.

11. detected_language (string) — The language of the voter's message. Must be exactly one of: "malay", "english", "tamil", "mandarin", "other". Use "malay" for Bahasa Malaysia, "english" for English, "tamil" for Tamil, "mandarin" for Mandarin Chinese, "other" for any other language.

Respond ONLY with valid JSON. Do not include markdown, code blocks, or any text outside the JSON object."#;

impl LlmClient {
    pub fn new(endpoint: &str, model: &str, api_key: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub async fn analyze(&self, raw_text: &str, context: &Option<String>) -> anyhow::Result<LlmAnalysis> {
        let user_message = match context {
            Some(ctx) => format!("Parent context:\n{}\n\nVoter message:\n{}", ctx, raw_text),
            None => format!("Voter message:\n{}", raw_text),
        };

        let mut messages = vec![
            Message {
                role: "system".into(),
                content: SYSTEM_PROMPT.into(),
            },
            Message {
                role: "user".into(),
                content: user_message,
            },
        ];

        for attempt in 0..3 {
            info!(attempt, "LLM analyze attempt");

            let content = match self.call_llm_raw(&messages).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(attempt, error = %e, "LLM request failed");
                    if attempt < 2 {
                        continue;
                    }
                    anyhow::bail!("LLM request failed after 3 attempts: {e}");
                }
            };

            let parsed = match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(v) => v,
                Err(e) => {
                    warn!(attempt, error = %e, raw = %content, "LLM response invalid JSON");
                    if attempt < 2 {
                        messages.push(Message {
                            role: "assistant".into(),
                            content: content.clone(),
                        });
                        messages.push(Message {
                            role: "user".into(),
                            content: format!("Response was not valid JSON: {e}. Respond with ONLY valid JSON matching the requested schema."),
                        });
                        continue;
                    }
                    error!(attempt, "LLM failed to return valid JSON after 3 attempts");
                    return Err(anyhow::anyhow!("LLM failed to return valid JSON after 3 attempts: {e}"));
                }
            };

            let missing = missing_required_fields(&parsed);
            if !missing.is_empty() {
                warn!(attempt, missing = ?missing, "LLM response missing required fields");
                if attempt < 2 {
                    messages.push(Message {
                        role: "assistant".into(),
                        content: content.clone(),
                    });
                    messages.push(Message {
                        role: "user".into(),
                        content: format!("Response is missing required fields: {}. Provide a complete response with all fields.", missing.join(", ")),
                    });
                    continue;
                }
                error!(attempt, missing = ?missing, "LLM still missing fields after 3 attempts, using defaults");
            }

            return Ok(LlmAnalysis::from_json_value(&parsed));
        }

        anyhow::bail!("LLM failed to produce valid analysis after 3 attempts")
    }

    pub async fn generate_embedding(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let body = serde_json::json!({
            "model": self.model,
            "input": text,
        });

        let resp = self
            .client
            .post(format!("{}/embeddings", self.endpoint))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM embedding request failed ({}): {}", status, body);
        }

        let data: serde_json::Value = resp.json().await?;
        let embedding = data["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("LLM embedding response missing data[0].embedding"))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    async fn call_llm_raw(&self, messages: &[Message]) -> anyhow::Result<String> {
        let request = LlmRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            response_format: ResponseFormat {
                format_type: "json_schema".into(),
                json_schema: serde_json::json!({
                    "name": "LlmAnalysis",
                    "strict": true,
                    "schema": {}
                }),
            },
        };

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.endpoint))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM request failed ({}): {}", status, body);
        }

        let llm_resp: LlmResponse = resp.json().await?;
        let content = llm_resp
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("LLM returned no choices"))?
            .message
            .content
            .clone();

        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_llm_client() -> Option<LlmClient> {
        let api_key = std::env::var("OPENROUTER_API_KEY").ok()?;
        let endpoint = std::env::var("OPENROUTER_BASE_URL")
            .unwrap_or_else(|_| "https://openrouter.ai/api/v1".into());
        let model = std::env::var("LLM_MODEL")
            .unwrap_or_else(|_| "openai/gpt-oss-120b".into());
        Some(LlmClient::new(&endpoint, &model, &api_key))
    }

    macro_rules! skip_unless_openrouter {
        () => {
            match get_llm_client() {
                Some(c) => c,
                None => return,
            }
        };
    }

    #[tokio::test]
    async fn test_analyze_local_infrastructure_complaint() {
        let client = skip_unless_openrouter!();
        let result = client
            .analyze("Jalan ray dekat Tmn Mawar berlubang teruk, dah 3 bulan x dibaiki. Bahaya untuk budak sekolah.", &None)
            .await
            .unwrap();
        assert!(result.has_substantive_value);
        assert!(result.rejection_reason.is_none());
        assert_eq!(result.intent_type, "local_issue");
        assert_eq!(result.primary_category, "infrastructure");
        assert!(result.urgency == "high" || result.urgency == "medium");
        assert!(!result.cleaned_summary.is_empty());
        assert!(!result.sub_categories.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_policy_agenda() {
        let client = skip_unless_openrouter!();
        let result = client
            .analyze("Kerajaan patut naikkan gaji minimum kepada RM2,500 dan buat undang-undang kerja hibrid untuk sektor awam dan swasta.", &None)
            .await
            .unwrap();
        assert!(result.has_substantive_value);
        assert_eq!(result.intent_type, "policy_agenda");
        assert_eq!(result.scope, "national");
        assert_eq!(result.primary_category, "economy_and_labor");
        assert_eq!(result.voter_sentiment, "demanding");
    }

    #[tokio::test]
    async fn test_analyze_noise_greeting() {
        let client = skip_unless_openrouter!();
        let result = client
            .analyze("Selamat pagi", &None)
            .await
            .unwrap();
        assert!(!result.has_substantive_value);
        assert!(result.rejection_reason.is_some());
        assert_eq!(result.intent_type, "noise");
    }

    #[tokio::test]
    async fn test_analyze_with_context_resolves_reply() {
        let client = skip_unless_openrouter!();
        let context = "Kerajaan negeri akan menaikkan taraf sistem longkang di Taman Mawar dengan peruntukan RM5 juta."
            .to_string();
        let result = client
            .analyze("Setuju sangat, dah lama tunggu.", &Some(context))
            .await
            .unwrap();
        assert!(result.has_substantive_value);
        assert_eq!(result.intent_type, "policy_agenda");
        assert_eq!(result.primary_category, "infrastructure");
        assert_eq!(result.voter_sentiment, "supportive");
    }

    #[tokio::test]
    async fn test_analyze_cleaned_summary_normalizes_slang() {
        let client = skip_unless_openrouter!();
        let result = client
            .analyze("Jln ray kat sini berlubang gile, mntk tlong repair lmbt sgt nih", &None)
            .await
            .unwrap();
        assert!(result.has_substantive_value);
        let s = result.cleaned_summary.to_lowercase();
        assert!(s.contains("jalan") || s.contains("road"), "expected cleaned summary to contain jalan/road, got: {}", s);
        assert!(!s.contains("gile"), "expected slang 'gile' removed, got: {}", s);
    }

    #[tokio::test]
    async fn test_analyze_all_enum_fields_valid() {
        let client = skip_unless_openrouter!();
        let result = client
            .analyze("Klinik kesihatan di sini kekurangan doktor, pesakit terpaksa tunggu 6 jam.", &None)
            .await
            .unwrap();
        assert!(result.has_substantive_value);
        assert!(matches!(result.intent_type.as_str(), "local_issue" | "policy_agenda"));
        assert!(matches!(result.scope.as_str(), "local" | "state" | "national"));
        assert!(matches!(result.urgency.as_str(), "low" | "medium" | "high"));
        assert!(matches!(result.voter_sentiment.as_str(), "supportive" | "frustrated" | "neutral" | "demanding"));
    }

    #[tokio::test]
    async fn test_analyze_empty_location_tags() {
        let client = skip_unless_openrouter!();
        let result = client
            .analyze("Saya sokong dasar pendidikan percuma", &None)
            .await
            .unwrap();
        assert!(result.has_substantive_value);
        assert!(result.inferred_location_tags.is_empty());
    }

    #[tokio::test]
    async fn test_llm_http_error() {
        let client = LlmClient::new("http://localhost:1", "model", "bad-key");
        let result = client.analyze("test", &None).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_from_json_value_all_fields() {
        let json = serde_json::json!({
            "has_substantive_value": true,
            "rejection_reason": null,
            "intent_type": "local_issue",
            "scope": "local",
            "cleaned_summary": "Road in Taman Mawar has potholes.",
            "primary_category": "infrastructure",
            "sub_categories": ["potholes", "road_safety"],
            "urgency": "high",
            "voter_sentiment": "frustrated",
            "inferred_location_tags": ["Taman Mawar"],
            "detected_language": "malay"
        });
        let a = LlmAnalysis::from_json_value(&json);
        assert!(a.has_substantive_value);
        assert_eq!(a.intent_type, "local_issue");
        assert_eq!(a.primary_category, "infrastructure");
        assert_eq!(a.sub_categories.len(), 2);
        assert_eq!(a.voter_sentiment, "frustrated");
        assert_eq!(a.detected_language, "malay");
    }

    #[test]
    fn test_from_json_value_missing_fields_defaults() {
        let json = serde_json::json!({});
        let a = LlmAnalysis::from_json_value(&json);
        assert!(a.has_substantive_value);
        assert_eq!(a.intent_type, "noise");
        assert_eq!(a.scope, "local");
        assert_eq!(a.voter_sentiment, "neutral");
        assert_eq!(a.detected_language, "other");
        assert!(a.sub_categories.is_empty());
        assert!(a.inferred_location_tags.is_empty());
    }

    #[test]
    fn test_missing_required_fields_identifies_omissions() {
        let json = serde_json::json!({
            "has_substantive_value": true,
            "intent_type": "local_issue"
        });
        let missing = missing_required_fields(&json);
        assert!(missing.contains(&"scope".to_string()));
        assert!(missing.contains(&"voter_sentiment".to_string()));
        assert!(missing.contains(&"cleaned_summary".to_string()));
        assert!(!missing.contains(&"has_substantive_value".to_string()));
        assert!(!missing.contains(&"intent_type".to_string()));
    }

    #[test]
    fn test_missing_required_fields_all_present() {
        let json = serde_json::json!({
            "has_substantive_value": true,
            "rejection_reason": null,
            "intent_type": "local_issue",
            "scope": "local",
            "cleaned_summary": "test",
            "primary_category": "infrastructure",
            "sub_categories": [],
            "urgency": "low",
            "voter_sentiment": "neutral",
            "inferred_location_tags": [],
            "detected_language": "malay"
        });
        let missing = missing_required_fields(&json);
        assert!(missing.is_empty());
    }
}
