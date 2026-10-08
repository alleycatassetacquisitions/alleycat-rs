ALTER TYPE player_mode RENAME TO player_team;
ALTER TABLE players RENAME COLUMN mode TO team;

-- Recreate the NOT NULL constraint with the column's current name (PostgreSQL 18).
ALTER TABLE players ALTER COLUMN team DROP NOT NULL;
ALTER TABLE players ALTER COLUMN team SET NOT NULL;
