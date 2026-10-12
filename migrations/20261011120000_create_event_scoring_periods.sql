-- Shared PostgreSQL capability, also supported by the managed database.
-- Keep the extension installed across application resets: other objects may use it.
CREATE EXTENSION IF NOT EXISTS btree_gist;

CREATE TABLE event_scoring_periods (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id uuid NOT NULL REFERENCES events(id),
    started_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    closes_at timestamptz,
    CONSTRAINT event_scoring_periods_finite_times CHECK (
        isfinite(started_at) AND (closes_at IS NULL OR isfinite(closes_at))
    ),
    CONSTRAINT event_scoring_periods_time_order CHECK (closes_at > started_at),
    -- [start, close): adjacent periods are allowed, overlapping periods are not.
    -- NULL is an unbounded upper end, so this also permits only one open period.
    CONSTRAINT event_scoring_periods_no_overlap EXCLUDE USING gist (
        event_id WITH =,
        tstzrange(started_at, closes_at, '[)') WITH &&
    )
);

COMMENT ON TABLE event_scoring_periods IS
    'Event scoring history. Opening creates a row; closing fills closes_at. Match eligibility uses device occurrence time, not upload time. No unclosed period means scoring is disabled.';
COMMENT ON COLUMN event_scoring_periods.started_at IS
    'Inclusive scoring start instant. Stored as timestamptz; devices report UTC.';
COMMENT ON COLUMN event_scoring_periods.closes_at IS
    'Exclusive scoring end instant. NULL means this period is still open. Closing a period preserves its history.';
