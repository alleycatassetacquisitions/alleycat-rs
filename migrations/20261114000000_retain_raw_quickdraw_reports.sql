ALTER TYPE quickdraw_scoring_status ADD VALUE 'tie';
ALTER TABLE quickdraw_matches ALTER COLUMN winner_player_id DROP NOT NULL;
ALTER TABLE quickdraw_matches DROP CONSTRAINT quickdraw_matches_scoring_teams_check;
ALTER TABLE quickdraw_matches ADD CONSTRAINT quickdraw_matches_scoring_teams_check CHECK (
    (winner_player_id IS NULL AND scoring_status::text = 'tie') OR
    (winner_player_id IS NOT NULL AND (
        scoring_status::text IN ('scoring_disabled', 'delayed_scores_disabled') OR
        scoring_status = CASE
            WHEN player_a_team_id IS NULL OR player_b_team_id IS NULL
                THEN 'unassigned_team'::quickdraw_scoring_status
            WHEN player_a_team_id = player_b_team_id THEN 'same_team'::quickdraw_scoring_status
            ELSE 'scoring'::quickdraw_scoring_status
        END
    ))
);
COMMENT ON TABLE quickdraw_matches IS NULL;
COMMENT ON COLUMN quickdraw_matches.winner_player_id IS NULL;
COMMENT ON COLUMN quickdraw_matches.scoring_status IS NULL;

CREATE TABLE quickdraw_matches_raw (
    id uuid PRIMARY KEY,
    event_id uuid NOT NULL REFERENCES events(id),
    match_id uuid,
    mac_address text,
    received_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    submitted_match_id text NOT NULL,
    player_a_id text NOT NULL,
    player_b_id text NOT NULL,
    player_a_team_id text,
    player_b_team_id text,
    player_a_draw_time_ms numeric(20, 0) CHECK (player_a_draw_time_ms BETWEEN 0 AND 18446744073709551615),
    player_b_draw_time_ms numeric(20, 0) CHECK (player_b_draw_time_ms BETWEEN 0 AND 18446744073709551615),
    winner_player_id text,
    occurred_at_unix_ms bigint,
    failure_reason text,
    submitted_mac_address text,
    outcome text CHECK (outcome IN ('completed', 'tie', 'failed')),
    comparison_key text
);
CREATE INDEX quickdraw_matches_raw_event_received_idx
    ON quickdraw_matches_raw (event_id, received_at DESC, id DESC);
CREATE INDEX quickdraw_matches_raw_event_match_idx
    ON quickdraw_matches_raw (event_id, match_id);
