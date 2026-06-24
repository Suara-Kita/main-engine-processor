CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS voter_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_identifier TEXT NOT NULL,
    source_channel TEXT NOT NULL,
    display_name TEXT,
    contact_info TEXT,
    inferred_constituency TEXT,
    first_interaction_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_interaction_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    interaction_count INT NOT NULL DEFAULT 1,
    metadata JSONB DEFAULT '{}'
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_voter_profiles_client ON voter_profiles (client_identifier);

CREATE TABLE IF NOT EXISTS interactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ingestion_id UUID NOT NULL UNIQUE,
    voter_profile_id UUID REFERENCES voter_profiles(id),
    source_channel TEXT NOT NULL,
    raw_text TEXT NOT NULL,
    content_type TEXT NOT NULL DEFAULT 'text_only',
    media_attachments TEXT[] DEFAULT '{}',
    context_anchor JSONB,
    intent_type TEXT,
    scope TEXT,
    primary_category TEXT,
    sub_categories TEXT[] DEFAULT '{}',
    cleaned_summary TEXT,
    urgency TEXT,
    voter_sentiment TEXT,
    inferred_location_tags TEXT[] DEFAULT '{}',
    rejection_reason TEXT,
    raw_language TEXT NOT NULL DEFAULT 'other'
        CHECK (raw_language IN ('malay', 'english', 'tamil', 'mandarin', 'other')),
    constituency TEXT,
    response_id VARCHAR,
    response_text TEXT,
    marked BOOLEAN NOT NULL DEFAULT FALSE,
    dispatch_error TEXT,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'rejected', 'noise', 'dispatched', 'dispatch_error')),
    ingested_at TIMESTAMPTZ NOT NULL,
    processed_at TIMESTAMPTZ,
    approved_at TIMESTAMPTZ,
    dispatched_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_interactions_status ON interactions (status);
CREATE INDEX IF NOT EXISTS idx_interactions_ingestion_id ON interactions (ingestion_id);
CREATE INDEX IF NOT EXISTS idx_interactions_ingested_at ON interactions (ingested_at DESC);
CREATE INDEX IF NOT EXISTS idx_interactions_primary_category ON interactions (primary_category);
CREATE INDEX IF NOT EXISTS idx_interactions_urgency ON interactions (urgency);

CREATE TABLE IF NOT EXISTS issue_embeddings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    interaction_id UUID NOT NULL REFERENCES interactions(id),
    embedding vector(1536),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
