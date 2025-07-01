-- Add migration script here
CREATE TABLE IF NOT EXISTS submissions (
    id BIGSERIAL PRIMARY KEY,
    submission_id UUID NOT NULL,
    submission_type TEXT NOT NULL,
    session_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    status TEXT NOT NULL,
    result TEXT,
    reason_code TEXT,
    submission_data TEXT,
    request_data TEXT,
    nfc_identifier TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique__submission_id UNIQUE (submission_id),
    CONSTRAINT unique__session_id UNIQUE (session_id)
);
