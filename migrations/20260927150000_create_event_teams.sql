CREATE TABLE teams (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id uuid NOT NULL REFERENCES events(id),
    name text NOT NULL CHECK (name <> '' AND name = btrim(name)),
    CONSTRAINT teams_event_name_key UNIQUE (event_id, name),
    -- Composite key makes cross-event player membership impossible.
    CONSTRAINT teams_event_id_key UNIQUE (event_id, id)
);

-- Pre-release migrations target fresh databases; no membership backfill.
ALTER TABLE players
    DROP COLUMN team,
    ADD COLUMN team_id uuid,
    ADD CONSTRAINT players_event_team_fkey
        FOREIGN KEY (event_id, team_id) REFERENCES teams(event_id, id);
DROP TYPE player_team;
