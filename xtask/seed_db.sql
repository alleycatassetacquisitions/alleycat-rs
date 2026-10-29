BEGIN;

INSERT INTO events (id, name, created_at)
VALUES ('20000000-0000-4000-8000-000000000001', 'Development event', now())
ON CONFLICT (id) DO NOTHING;

UPDATE app_state
SET active_event_id = '20000000-0000-4000-8000-000000000001'
WHERE id = 1 AND active_event_id IS NULL;

-- Coordinate with registration before changing this event's players/counter.
SELECT id FROM events WHERE id = '20000000-0000-4000-8000-000000000001' FOR UPDATE;

-- Team identity is independent of its editable name. Reseeding must not rename
-- teams or recreate the default names after a rename.
INSERT INTO teams (id, event_id, name)
VALUES
    ('30000000-0000-4000-8000-000000000001', '20000000-0000-4000-8000-000000000001', 'Hunter'),
    ('30000000-0000-4000-8000-000000000002', '20000000-0000-4000-8000-000000000001', 'Bounty')
ON CONFLICT (id) DO NOTHING;

-- Optional development-event choices, not defaults for every event.
-- Stable IDs preserve category/affiliation renames when the seed is run again.
INSERT INTO affiliation_categories (id, event_id, name)
VALUES
    ('40000000-0000-4000-8000-000000000001', '20000000-0000-4000-8000-000000000001', 'Allegiance'),
    ('40000000-0000-4000-8000-000000000002', '20000000-0000-4000-8000-000000000001', 'Faction')
ON CONFLICT (id) DO NOTHING;

INSERT INTO affiliations (id, event_id, category_id, name)
VALUES
    ('50000000-0000-4000-8000-000000000001', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000001', 'Helix Industries'),
    ('50000000-0000-4000-8000-000000000002', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000001', 'Endline Solutions'),
    ('50000000-0000-4000-8000-000000000003', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000001', 'Reboot Syndicate'),
    ('50000000-0000-4000-8000-000000000101', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000002', 'Metamorphosis'),
    ('50000000-0000-4000-8000-000000000102', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000002', 'Sentinels'),
    ('50000000-0000-4000-8000-000000000103', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000002', 'Rapture'),
    ('50000000-0000-4000-8000-000000000104', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000002', 'Thirteens'),
    ('50000000-0000-4000-8000-000000000105', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000002', 'Corvid Alliance'),
    ('50000000-0000-4000-8000-000000000106', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000002', 'Vulturecorp'),
    ('50000000-0000-4000-8000-000000000107', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000002', 'Voidfarer''s Guild'),
    ('50000000-0000-4000-8000-000000000108', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000002', 'Sect of the Robot Moon'),
    ('50000000-0000-4000-8000-000000000109', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000002', 'Transient Resistance'),
    ('50000000-0000-4000-8000-000000000110', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000002', 'Beautiful Chaos')
ON CONFLICT (id) DO NOTHING;

-- Leave player affiliations to the operator; reseeding never changes assignments.

-- Stable IDs make the development fixtures safe to update and run repeatedly.
-- Stable team IDs also assign new fixtures correctly after team renames.
-- Reseeding preserves existing membership.
INSERT INTO players (event_id, id, pdn_code, name, team_id, email, created_at)
VALUES
    (
        '20000000-0000-4000-8000-000000000001',
        '10000000-0000-4000-8000-000000000001',
        '0101',
        'Nyx Voltage',
        '30000000-0000-4000-8000-000000000001',
        'avery@example.test',
        now() - interval '45 days'
    ),
    (
        '20000000-0000-4000-8000-000000000001',
        '10000000-0000-4000-8000-000000000002',
        '0202',
        'Rook Zero',
        '30000000-0000-4000-8000-000000000002',
        'jordan@example.test',
        now() - interval '20 days'
    ),
    (
        '20000000-0000-4000-8000-000000000001',
        '10000000-0000-4000-8000-000000000003',
        '0303',
        'Echo Vane',
        NULL,
        NULL,
        now() - interval '7 days'
    ),
    (
        '20000000-0000-4000-8000-000000000001',
        '10000000-0000-4000-8000-000000000004',
        '0404',
        'Chrome Wraith',
        '30000000-0000-4000-8000-000000000001',
        'sam@example.test',
        now() - interval '2 days'
    ),
    (
        '20000000-0000-4000-8000-000000000001',
        '10000000-0000-4000-8000-000000000005',
        '0505',
        'Nova Static',
        '30000000-0000-4000-8000-000000000002',
        'taylor@example.test',
        now() - interval '4 hours'
    )
ON CONFLICT (id) DO UPDATE SET
    event_id = EXCLUDED.event_id,
    pdn_code = EXCLUDED.pdn_code,
    name = EXCLUDED.name,
    email = EXCLUDED.email,
    created_at = EXCLUDED.created_at;

-- Keep the original five players and add 45 with stable IDs and PINs.
-- Existing player edits and team memberships are preserved for these additions.
WITH fixtures (player_number, name, team_number) AS (
    VALUES
    (6, 'Chrome Reaper', 1),
    (7, 'Neon Samurai', 2),
    (8, 'Cyber Ghost', 2),
    (9, 'Pixel Punk', 1),
    (10, 'Razor Byte', 1),
    (11, 'Circuit Breaker', 1),
    (12, 'Glitch Hustler', 1),
    (13, 'Neon Ninja', 1),
    (14, 'Code Phantom', 1),
    (15, 'Data Shredder', 1),
    (16, 'Synth Oracle', 1),
    (17, 'Byte Bandit', 1),
    (18, 'Digital Drifter', 1),
    (19, 'Holo Hack', 1),
    (20, 'Matrix Marauder', 1),
    (21, 'Quantum Cowboy', 1),
    (22, 'Techno Trickster', 1),
    (23, 'Nano Nomad', 1),
    (24, 'Virtual Vandal', 1),
    (25, 'Electric Eel', 1),
    (26, 'Pixelated Pirate', 2),
    (27, 'Cyber Coyote', 2),
    (28, 'Neon Nomad', 2),
    (29, 'Circuit Clown', 2),
    (30, 'Glitch Guru', 2),
    (31, 'Silver Relay', 2),
    (32, 'Amber Flux', 2),
    (33, 'Cobalt Runner', 2),
    (34, 'Violet Vector', 2),
    (35, 'Copper Cipher', 2),
    (36, 'Crimson Pulse', 2),
    (37, 'Jade Signal', 2),
    (38, 'Indigo Spark', 2),
    (39, 'Golden Ghost', 2),
    (40, 'Scarlet Switch', 2),
    (41, 'Azure Echo', 2),
    (42, 'Ivory Circuit', 2),
    (43, 'Onyx Orbit', 2),
    (44, 'Ruby Reactor', 2),
    (45, 'Emerald Edge', 2),
    (46, 'Saffron Surge', 2),
    (47, 'Titanium Tide', 2),
    (48, 'Mercury Mirage', 2),
    (49, 'Plasma Pilot', 2),
    (50, 'Laser Lynx', 2)
)
INSERT INTO players (event_id, id, pdn_code, name, team_id, email, created_at)
SELECT
    '20000000-0000-4000-8000-000000000001'::uuid,
    ('10000000-0000-4000-8000-' || lpad(player_number::text, 12, '0'))::uuid,
    lpad(player_number::text, 4, '0'),
    name,
    ('30000000-0000-4000-8000-' || lpad(team_number::text, 12, '0'))::uuid,
    'player' || player_number || '@example.test',
    now() - interval '2 days'
FROM fixtures
ON CONFLICT (id) DO NOTHING;

-- Never rewind the allocation counter when fixtures are reapplied.
UPDATE events
SET next_pdn_code = GREATEST(next_pdn_code, (
    SELECT COALESCE(MAX(pdn_code::integer), 0) + 1
    FROM players WHERE event_id = events.id
))
WHERE id = '20000000-0000-4000-8000-000000000001';

-- 123 matches is the largest total that fits the requested distribution:
-- 2 players with 20, 5 with 15, 32 with 4, 1 with 3, and 10 with none.
-- Each unordered opponent pair appears once. Explicit pairings keep the data
-- reproducible without a random generator or a scheduling algorithm at seed time.
WITH opponents (player_a_number, player_b_numbers) AS (
    VALUES
    (1, ARRAY[2, 5, 7, 8, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41]),
    (4, ARRAY[2, 5, 7, 8, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41]),
    (6, ARRAY[2, 5, 7, 8, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36]),
    (9, ARRAY[2, 5, 7, 8]),
    (10, ARRAY[2, 5, 7, 8]),
    (11, ARRAY[2, 5, 7, 8]),
    (12, ARRAY[2, 5, 7, 8]),
    (13, ARRAY[2, 5, 7, 8]),
    (14, ARRAY[2, 5, 7, 8]),
    (15, ARRAY[2, 5, 7, 8]),
    (16, ARRAY[2, 5, 7, 8]),
    (17, ARRAY[2, 5, 7, 8]),
    (18, ARRAY[2, 5, 7, 8]),
    (19, ARRAY[2, 5, 7, 8]),
    (20, ARRAY[37, 38, 39, 40]),
    (21, ARRAY[2, 5, 7, 8]),
    (22, ARRAY[26, 27, 28, 29]),
    (23, ARRAY[30, 31, 32, 33]),
    (24, ARRAY[34, 35, 36, 37]),
    (25, ARRAY[38, 39, 40, 41])
), pairings AS (
    SELECT player_a_number, player_b_number,
           row_number() OVER (ORDER BY player_a_number, player_b_number) AS match_number
    FROM opponents
    CROSS JOIN LATERAL unnest(player_b_numbers) AS opponent(player_b_number)
), fixtures AS (
    SELECT pairings.*, a.id AS player_a_id, b.id AS player_b_id,
           a.team_id AS player_a_team_id, b.team_id AS player_b_team_id,
           200 + (player_a_number * 37 + player_b_number * 19) % 600 AS time_a,
           201 + (player_a_number * 23 + player_b_number * 41) % 600 AS time_b
    FROM pairings
    JOIN players a ON a.id =
        ('10000000-0000-4000-8000-' || lpad(player_a_number::text, 12, '0'))::uuid
    JOIN players b ON b.id =
        ('10000000-0000-4000-8000-' || lpad(player_b_number::text, 12, '0'))::uuid
)
-- These historical results carry their own scoring decisions. Seeding does not
-- change scoring settings or periods. Reruns preserve every accepted result,
-- including its original timestamp and team snapshots after membership edits.
INSERT INTO quickdraw_matches (
    id, event_id, player_a_id, player_b_id, player_a_team_id, player_b_team_id,
    player_a_draw_time_ms, player_b_draw_time_ms, winner_player_id,
    occurred_at, scoring_status
)
SELECT
    ('60000000-0000-4000-8000-' || lpad((player_a_number * 100 + player_b_number)::text, 12, '0'))::uuid,
    '20000000-0000-4000-8000-000000000001'::uuid,
    player_a_id, player_b_id, player_a_team_id, player_b_team_id,
    time_a, time_b,
    CASE WHEN time_a <= time_b THEN player_a_id ELSE player_b_id END,
    now() - interval '3 hours' + match_number * interval '1 minute',
    (CASE
        WHEN player_a_team_id IS NULL OR player_b_team_id IS NULL THEN 'unassigned_team'
        WHEN player_a_team_id = player_b_team_id THEN 'same_team'
        ELSE 'scoring'
    END)::quickdraw_scoring_status
FROM fixtures
ON CONFLICT (id) DO NOTHING;

INSERT INTO device_logs (
    device_mac,
    crash_number,
    uptime_ms,
    received_at,
    software_version,
    reset_reason,
    program_counter,
    exception_cause,
    task_name
)
VALUES
    (
        '02:00:00:00:01:01', 1, 3421, now() - interval '6 days',
        '0.9.3', 1, 1074270772, 6, 'main'
    ),
    (
        '02:00:00:00:01:01', 2, 86422000, now() - interval '4 days',
        '0.9.3', 2, 1074272800, 28, 'radio'
    ),
    (
        '02:00:00:00:01:01', 3, 1250, now() - interval '18 hours',
        '0.10.0', 3, NULL, NULL, NULL
    ),
    (
        '02:00:00:00:01:01', 4, 9412233, now() - interval '35 minutes',
        '0.10.0', 1, 1074302992, 9, 'display'
    ),
    (
        '02:00:00:00:02:02', 1, 7200042, now() - interval '5 days',
        '0.9.3', 4, 1074598452, 3, 'network'
    ),
    (
        '02:00:00:00:02:02', 2, 18234567, now() - interval '3 days',
        '0.10.0', 2, 1074270772, 6, 'radio'
    ),
    (
        '02:00:00:00:02:02', 3, 420, now() - interval '1 day',
        '0.10.0', 5, NULL, NULL, 'main'
    ),
    (
        '02:00:00:00:02:02', 4, 55901234, now() - interval '12 minutes',
        '0.10.1-dev', 1, 1074272800, 28, 'sensor'
    ),
    (
        '02:00:00:00:03:03', 1, 300125, now() - interval '30 days',
        NULL, 3, NULL, NULL, NULL
    ),
    (
        '02:00:00:00:03:03', 2, 129600000, now() - interval '10 days',
        '0.9.3', 2, 1074302992, 9, 'display'
    ),
    (
        '02:00:00:00:03:03', 3, 7712345, now() - interval '2 hours',
        '0.10.0', 4, 1074598452, 3, 'network'
    ),
    (
        '02:00:00:00:03:03', 4, 98765, now() - interval '5 minutes',
        '0.10.1-dev', 1, 1074270772, 6, 'main'
    )
ON CONFLICT (device_mac, crash_number) DO UPDATE SET
    uptime_ms = EXCLUDED.uptime_ms,
    received_at = EXCLUDED.received_at,
    software_version = EXCLUDED.software_version,
    reset_reason = EXCLUDED.reset_reason,
    program_counter = EXCLUDED.program_counter,
    exception_cause = EXCLUDED.exception_cause,
    task_name = EXCLUDED.task_name;

COMMIT;
