-- create events to separate scoring for each game run
CREATE TABLE events (
    id uuid not null,
    PRIMARY KEY (id),
    name text not null,
    venue_name text,
    -- 10000 represents exhaustion of the four-digit code space.
    next_pdn_code integer NOT NULL DEFAULT 1
        CHECK (next_pdn_code BETWEEN 1 AND 10000),
    created_at timestamptz not null
);

-- Global exclusions apply to future assignments in every event.
-- Adding a reservation does not change codes already assigned to players.
CREATE TABLE reserved_pdn_codes (
    code text PRIMARY KEY CHECK (code ~ '^[0-9]{4}$'),
    reason text
);

INSERT INTO reserved_pdn_codes (code)
VALUES
    ('0000'),
    ('1111'),
    ('2222'),
    ('3333'),
    ('4444'),
    ('5555'),
    ('6666'),
    ('7777'),
    ('8888'),
    ('9999'),
    ('6969'),
    ('8008'),
    ('1337');

-- Registration will allocate sequentially from the event's counter, skipping
-- reserved codes, instead of consuming randomly selected codes from a pool.
DROP TABLE available_pdn_codes;

CREATE TABLE app_state (
    id integer PRIMARY KEY CHECK (id = 1),
    active_event_id uuid REFERENCES events(id)
);

-- Requires an empty players table; clear existing test players before applying.
ALTER TABLE players
ADD COLUMN event_id uuid NOT NULL REFERENCES events(id);

-- A fresh database has no active event until one is created through the API.
INSERT INTO app_state (id, active_event_id)
VALUES (1, NULL);

-- Every player now belongs to an event. Names and codes may be reused in
-- different events, but must remain unique within the same event.
ALTER TABLE players
    DROP CONSTRAINT players_name_key,
    DROP CONSTRAINT players_pdn_code_key,
    ADD CONSTRAINT players_event_name_key UNIQUE (event_id, name),
    ADD CONSTRAINT players_event_pdn_code_key UNIQUE (event_id, pdn_code);

-- Support listing one event's players in newest-first order.
CREATE INDEX players_event_created_at_id_idx
ON players (event_id, created_at DESC, id DESC);
