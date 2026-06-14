use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceChannel {
    Whatsapp,
    Telegram,
    FacebookCrawler,
    NewsCrawler,
    WebPortal,
    F2fStaff,
}

impl SourceChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Whatsapp => "whatsapp",
            Self::Telegram => "telegram",
            Self::FacebookCrawler => "facebook_crawler",
            Self::NewsCrawler => "news_crawler",
            Self::WebPortal => "web_portal",
            Self::F2fStaff => "f2f_staff",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    TextOnly,
    Image,
    Video,
    Document,
    Audio,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TextOnly => "text_only",
            Self::Image => "image",
            Self::Video => "video",
            Self::Document => "document",
            Self::Audio => "audio",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineMetadata {
    pub ingestion_id: Uuid,
    pub source_channel: SourceChannel,
    pub ingested_at: DateTime<Utc>,
    pub trace_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceProfile {
    pub client_identifier: String,
    pub display_name: Option<String>,
    pub contact_info: Option<String>,
    pub inferred_constituency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPayload {
    pub raw_text: String,
    pub content_type: ContentType,
    pub media_attachments: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContextAnchor {
    pub is_reply_or_comment: bool,
    pub parent_id: String,
    pub parent_title: String,
    pub parent_raw_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoterInput {
    pub pipeline_metadata: PipelineMetadata,
    pub source_profile: SourceProfile,
    pub content_payload: ContentPayload,
    pub context_anchor: Option<ContextAnchor>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_deserialize_whatsapp_message() {
        let json = json!({
            "pipeline_metadata": {
                "ingestion_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
                "source_channel": "whatsapp",
                "ingested_at": "2026-06-10T12:00:00Z",
                "trace_url": null
            },
            "source_profile": {
                "client_identifier": "+60123456789",
                "display_name": "Ahmad",
                "contact_info": null,
                "inferred_constituency": "P.102 Bangi"
            },
            "content_payload": {
                "raw_text": "Jalan berlubang",
                "content_type": "text_only",
                "media_attachments": []
            },
            "context_anchor": null
        });
        let input: VoterInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.pipeline_metadata.ingestion_id.to_string(), "f47ac10b-58cc-4372-a567-0e02b2c3d479");
        assert!(matches!(input.pipeline_metadata.source_channel, SourceChannel::Whatsapp));
        assert_eq!(input.source_profile.client_identifier, "+60123456789");
        assert_eq!(input.content_payload.raw_text, "Jalan berlubang");
        assert!(input.context_anchor.is_none());
    }

    #[test]
    fn test_deserialize_all_source_channels() {
        for (channel_str, variant) in [
            ("whatsapp", SourceChannel::Whatsapp),
            ("telegram", SourceChannel::Telegram),
            ("facebook_crawler", SourceChannel::FacebookCrawler),
            ("news_crawler", SourceChannel::NewsCrawler),
            ("web_portal", SourceChannel::WebPortal),
            ("f2f_staff", SourceChannel::F2fStaff),
        ] {
            let json = json!({
                "pipeline_metadata": {
                    "ingestion_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
                    "source_channel": channel_str,
                    "ingested_at": "2026-06-10T12:00:00Z",
                    "trace_url": null
                },
                "source_profile": {
                    "client_identifier": "test_id",
                    "display_name": null,
                    "contact_info": null,
                    "inferred_constituency": null
                },
                "content_payload": {
                    "raw_text": "test",
                    "content_type": "text_only",
                    "media_attachments": []
                },
                "context_anchor": null
            });
            let input: VoterInput = serde_json::from_value(json).unwrap();
            assert_eq!(
                std::mem::discriminant(&input.pipeline_metadata.source_channel),
                std::mem::discriminant(&variant),
            );
            assert_eq!(input.pipeline_metadata.source_channel.as_str(), channel_str);
        }
    }

    #[test]
    fn test_deserialize_all_content_types() {
        for (ct_str, variant) in [
            ("text_only", ContentType::TextOnly),
            ("image", ContentType::Image),
            ("video", ContentType::Video),
            ("document", ContentType::Document),
            ("audio", ContentType::Audio),
        ] {
            let json = json!({
                "pipeline_metadata": {
                    "ingestion_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
                    "source_channel": "telegram",
                    "ingested_at": "2026-06-10T12:00:00Z",
                    "trace_url": null
                },
                "source_profile": {
                    "client_identifier": "test_id",
                    "display_name": null,
                    "contact_info": null,
                    "inferred_constituency": null
                },
                "content_payload": {
                    "raw_text": "test",
                    "content_type": ct_str,
                    "media_attachments": []
                },
                "context_anchor": null
            });
            let input: VoterInput = serde_json::from_value(json).unwrap();
            assert_eq!(
                std::mem::discriminant(&input.content_payload.content_type),
                std::mem::discriminant(&variant),
            );
            assert_eq!(input.content_payload.content_type.as_str(), ct_str);
        }
    }

    #[test]
    fn test_deserialize_with_context_anchor() {
        let json = json!({
            "pipeline_metadata": {
                "ingestion_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
                "source_channel": "facebook_crawler",
                "ingested_at": "2026-06-10T12:00:00Z",
                "trace_url": "https://facebook.com/post/123"
            },
            "source_profile": {
                "client_identifier": "fb_user_1",
                "display_name": null,
                "contact_info": null,
                "inferred_constituency": null
            },
            "content_payload": {
                "raw_text": "Setuju",
                "content_type": "text_only",
                "media_attachments": []
            },
            "context_anchor": {
                "is_reply_or_comment": true,
                "parent_id": "fb_post_9988",
                "parent_title": "Naik taraf jalan",
                "parent_raw_text": "Kami akan naik taraf jalan"
            }
        });
        let input: VoterInput = serde_json::from_value(json).unwrap();
        let ctx = input.context_anchor.unwrap();
        assert!(ctx.is_reply_or_comment);
        assert_eq!(ctx.parent_id, "fb_post_9988");
        assert_eq!(ctx.parent_title, "Naik taraf jalan");
        assert_eq!(ctx.parent_raw_text, "Kami akan naik taraf jalan");
    }

    #[test]
    fn test_deserialize_with_media_attachments() {
        let json = json!({
            "pipeline_metadata": {
                "ingestion_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
                "source_channel": "telegram",
                "ingested_at": "2026-06-10T12:00:00Z",
                "trace_url": null
            },
            "source_profile": {
                "client_identifier": "tg_1",
                "display_name": null,
                "contact_info": null,
                "inferred_constituency": null
            },
            "content_payload": {
                "raw_text": "Longkang tersumbat",
                "content_type": "image",
                "media_attachments": ["https://storage.example.com/photo1.jpg", "https://storage.example.com/photo2.jpg"]
            },
            "context_anchor": null
        });
        let input: VoterInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.content_payload.media_attachments.len(), 2);
    }

    #[test]
    fn test_deserialize_missing_required_field_fails() {
        let json = json!({
            "pipeline_metadata": {
                "source_channel": "telegram",
                "ingested_at": "2026-06-10T12:00:00Z",
                "trace_url": null
            },
            "source_profile": {
                "client_identifier": "tg_1"
            },
            "content_payload": {
                "raw_text": "test",
                "content_type": "text_only",
                "media_attachments": []
            }
        });
        let result: Result<VoterInput, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_invalid_enum_fails() {
        let json = json!({
            "pipeline_metadata": {
                "ingestion_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
                "source_channel": "invalid_channel",
                "ingested_at": "2026-06-10T12:00:00Z",
                "trace_url": null
            },
            "source_profile": {
                "client_identifier": "tg_1",
                "display_name": null,
                "contact_info": null,
                "inferred_constituency": null
            },
            "content_payload": {
                "raw_text": "test",
                "content_type": "text_only",
                "media_attachments": []
            },
            "context_anchor": null
        });
        let result: Result<VoterInput, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}
