ALTER TABLE events ADD COLUMN accepting_delayed_scores boolean NOT NULL DEFAULT true;

ALTER TYPE quickdraw_scoring_status ADD VALUE 'delayed_scores_disabled';

ALTER TABLE quickdraw_matches DROP CONSTRAINT quickdraw_matches_scoring_teams_check;
ALTER TABLE quickdraw_matches ADD CONSTRAINT quickdraw_matches_scoring_teams_check CHECK (
    -- Compare as text so the newly added enum value is not used until commit.
    scoring_status::text IN ('scoring_disabled', 'delayed_scores_disabled') OR
    scoring_status = CASE
        WHEN player_a_team_id IS NULL OR player_b_team_id IS NULL
            THEN 'unassigned_team'::quickdraw_scoring_status
        WHEN player_a_team_id = player_b_team_id THEN 'same_team'::quickdraw_scoring_status
        ELSE 'scoring'::quickdraw_scoring_status
    END
);

COMMENT ON COLUMN events.accepting_delayed_scores IS
    'Reversible per-event switch, default true. When false, new matches from closed scoring periods are recorded as delayed_scores_disabled. Open-period scoring and previously accepted results are unchanged.';
COMMENT ON COLUMN quickdraw_matches.scoring_status IS
    'First accepted scoring decision: outside a scoring period -> scoring_disabled; closed period with delayed scores disabled -> delayed_scores_disabled; either team missing -> unassigned_team; equal teams -> same_team; otherwise scoring. Ingestion checks period history and the event switch under the event row lock. The database checks team consistency; duplicates never rescore.';
