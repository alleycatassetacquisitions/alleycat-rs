ALTER TABLE players ADD COLUMN active boolean NOT NULL DEFAULT true;

COMMENT ON COLUMN players.active IS
    'Reversible registration/device status, initially true. Inactive players retain their PIN, name, event, and match history and remain discoverable. This flag does not change accepted scores or reject delayed match uploads.';
