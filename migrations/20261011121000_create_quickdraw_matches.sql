-- Allow match participants to be constrained to the match's event.
ALTER TABLE players ADD CONSTRAINT players_event_id_key UNIQUE (event_id, id);

CREATE TYPE quickdraw_scoring_status AS ENUM (
    'scoring', 'same_team', 'scoring_disabled', 'unassigned_team'
);

CREATE TABLE quickdraw_matches (
    -- Shared device-generated duel ID. No server default: both devices must
    -- submit the same ID, regardless of which one uploads first.
    id uuid PRIMARY KEY,
    event_id uuid NOT NULL REFERENCES events(id),
    player_a_id uuid NOT NULL,
    player_b_id uuid NOT NULL,
    player_a_team_id uuid,
    player_b_team_id uuid,
    player_a_draw_time_ms bigint NOT NULL CHECK (player_a_draw_time_ms >= 0),
    player_b_draw_time_ms bigint NOT NULL CHECK (player_b_draw_time_ms >= 0),
    winner_player_id uuid NOT NULL,
    occurred_at timestamptz NOT NULL CHECK (isfinite(occurred_at)),
    received_at timestamptz NOT NULL DEFAULT clock_timestamp() CHECK (isfinite(received_at)),
    -- Ingestion decides eligibility from match time and team snapshots.
    -- Require an explicit decision; do not retain a scoring-period reference.
    scoring_status quickdraw_scoring_status NOT NULL,
    CONSTRAINT quickdraw_matches_scoring_teams_check CHECK (
        scoring_status = 'scoring_disabled' OR
        scoring_status = CASE
            WHEN player_a_team_id IS NULL OR player_b_team_id IS NULL
                THEN 'unassigned_team'::quickdraw_scoring_status
            WHEN player_a_team_id = player_b_team_id THEN 'same_team'::quickdraw_scoring_status
            ELSE 'scoring'::quickdraw_scoring_status
        END
    ),
    CONSTRAINT quickdraw_matches_distinct_players CHECK (player_a_id <> player_b_id),
    CONSTRAINT quickdraw_matches_winner_is_participant CHECK (
        winner_player_id IN (player_a_id, player_b_id)
    ),
    CONSTRAINT quickdraw_matches_player_a_fkey
        FOREIGN KEY (event_id, player_a_id) REFERENCES players(event_id, id),
    CONSTRAINT quickdraw_matches_player_b_fkey
        FOREIGN KEY (event_id, player_b_id) REFERENCES players(event_id, id),
    CONSTRAINT quickdraw_matches_team_a_fkey
        FOREIGN KEY (event_id, player_a_team_id) REFERENCES teams(event_id, id),
    CONSTRAINT quickdraw_matches_team_b_fkey
        FOREIGN KEY (event_id, player_b_team_id) REFERENCES teams(event_id, id)
);

CREATE INDEX quickdraw_matches_event_occurred_at_id_idx
    ON quickdraw_matches (event_id, occurred_at DESC, id DESC);
CREATE INDEX quickdraw_matches_player_a_history_idx
    ON quickdraw_matches (event_id, player_a_id, occurred_at, id);
CREATE INDEX quickdraw_matches_player_b_history_idx
    ON quickdraw_matches (event_id, player_b_id, occurred_at, id);

COMMENT ON TABLE quickdraw_matches IS
    'Completed two-player quickdraw results, including non-scoring duels. Ingestion must use INSERT ... ON CONFLICT (id) DO NOTHING: the first accepted report and its device timestamp win. No per-device reports or leaderboard totals are stored.';
COMMENT ON COLUMN quickdraw_matches.player_a_id IS
    'First participant slot, not a Hunter/Bounty role. Device reports must identify the same shared duel ID.';
COMMENT ON COLUMN quickdraw_matches.player_a_team_id IS
    'Team credited at match time, supplied by ingestion; NULL means unassigned. Historical snapshot, not a reference to current player membership.';
COMMENT ON COLUMN quickdraw_matches.player_b_team_id IS
    'Team credited at match time, supplied by ingestion; NULL means unassigned. Historical snapshot, not a reference to current player membership.';
COMMENT ON COLUMN quickdraw_matches.player_a_draw_time_ms IS
    'Device-measured draw duration in milliseconds. Nonnegative; zero is a measurement, not a missing-value sentinel.';
COMMENT ON COLUMN quickdraw_matches.player_b_draw_time_ms IS
    'Device-measured draw duration in milliseconds. Nonnegative; zero is a measurement, not a missing-value sentinel.';
COMMENT ON COLUMN quickdraw_matches.winner_player_id IS
    'Explicit device-reported winner; must be one of the two participants. No inferred winner from draw times.';
COMMENT ON COLUMN quickdraw_matches.occurred_at IS
    'Trusted UTC device timestamp from the first accepted upload. Use for scoring periods, daily boundaries and match chronology.';
COMMENT ON COLUMN quickdraw_matches.received_at IS
    'Defaults to the database clock at insertion. Ingestion must omit this field and preserve the first accepted result on duplicate uploads. Not the time used for scoring eligibility.';
COMMENT ON COLUMN quickdraw_matches.scoring_status IS
    'Scoring decision supplied by ingestion and stored with the first accepted result. Use event and device occurrence time to check scoring periods: no matching period -> scoring_disabled; either team missing -> unassigned_team; equal teams -> same_team; otherwise scoring. No period reference is retained. The database checks team consistency; period lookup belongs to ingestion.';
