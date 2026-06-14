use serde_json::json;
use uuid::Uuid;

use crate::models::voter_input::VoterInput;

pub fn sample_whatsapp_voter() -> VoterInput {
    serde_json::from_value(json!({
        "pipeline_metadata": {
            "ingestion_id": Uuid::new_v4(),
            "source_channel": "whatsapp",
            "ingested_at": "2026-06-10T12:00:00Z",
            "trace_url": null
        },
        "source_profile": {
            "client_identifier": "+60123456789",
            "display_name": "Ahmad Faizal",
            "contact_info": null,
            "inferred_constituency": "P.102 Bangi"
        },
        "content_payload": {
            "raw_text": "Jalan ray dekat Tmn Mawar berlubang teruk, dah 3 bulan x dibaiki.",
            "content_type": "text_only",
            "media_attachments": []
        },
        "context_anchor": null
    }))
    .unwrap()
}

pub fn sample_telegram_voter() -> VoterInput {
    serde_json::from_value(json!({
        "pipeline_metadata": {
            "ingestion_id": Uuid::new_v4(),
            "source_channel": "telegram",
            "ingested_at": "2026-06-10T12:05:00Z",
            "trace_url": null
        },
        "source_profile": {
            "client_identifier": "tg_987654321",
            "display_name": "Siti Nurhaliza",
            "contact_info": null,
            "inferred_constituency": null
        },
        "content_payload": {
            "raw_text": "Saya setuju dengan dasar kerajaan tentang pendidikan percuma",
            "content_type": "text_only",
            "media_attachments": []
        },
        "context_anchor": null
    }))
    .unwrap()
}

pub fn sample_voter_with_context() -> VoterInput {
    serde_json::from_value(json!({
        "pipeline_metadata": {
            "ingestion_id": Uuid::new_v4(),
            "source_channel": "facebook_crawler",
            "ingested_at": "2026-06-10T14:30:00Z",
            "trace_url": "https://facebook.com/groups/example/posts/123"
        },
        "source_profile": {
            "client_identifier": "fb_user_8821a9x",
            "display_name": null,
            "contact_info": null,
            "inferred_constituency": null
        },
        "content_payload": {
            "raw_text": "Betul sangat, dah lama tunggu",
            "content_type": "text_only",
            "media_attachments": []
        },
        "context_anchor": {
            "is_reply_or_comment": true,
            "parent_id": "fb_post_9988",
            "parent_title": "Naik taraf jalan Taman Mawar",
            "parent_raw_text": "Kerajaan negeri akan menaiktaraf jalan di Taman Mawar dengan peruntukan RM5 juta."
        }
    }))
    .unwrap()
}

pub fn sample_image_message() -> VoterInput {
    serde_json::from_value(json!({
        "pipeline_metadata": {
            "ingestion_id": Uuid::new_v4(),
            "source_channel": "telegram",
            "ingested_at": "2026-06-10T15:00:00Z",
            "trace_url": null
        },
        "source_profile": {
            "client_identifier": "tg_11223344",
            "display_name": "Kassim",
            "contact_info": null,
            "inferred_constituency": "N.24 Semenyih"
        },
        "content_payload": {
            "raw_text": "Longkang tersumbat, air naik sampai ke rumah",
            "content_type": "image",
            "media_attachments": ["https://storage.example.com/photos/longkang.jpg"]
        },
        "context_anchor": null
    }))
    .unwrap()
}

pub fn sample_voter_input_json() -> serde_json::Value {
    json!({
        "pipeline_metadata": {
            "ingestion_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
            "source_channel": "telegram",
            "ingested_at": "2026-06-10T12:00:00Z",
            "trace_url": null
        },
        "source_profile": {
            "client_identifier": "tg_123456",
            "display_name": "Test User",
            "contact_info": null,
            "inferred_constituency": null
        },
        "content_payload": {
            "raw_text": "Test message",
            "content_type": "text_only",
            "media_attachments": []
        },
        "context_anchor": null
    })
}
