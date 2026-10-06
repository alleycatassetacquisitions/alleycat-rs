BEGIN;

INSERT INTO events (id, name, created_at)
VALUES ('20000000-0000-4000-8000-000000000001', 'Development event', now())
ON CONFLICT (id) DO NOTHING;

UPDATE app_state
SET active_event_id = '20000000-0000-4000-8000-000000000001'
WHERE id = 1 AND active_event_id IS NULL;

-- Coordinate with registration before changing this event's players/counter.
SELECT id FROM events WHERE id = '20000000-0000-4000-8000-000000000001' FOR UPDATE;

-- Stable IDs make the development fixtures safe to update and run repeatedly.
INSERT INTO players (event_id, id, pdn_code, name, mode, email, created_at)
VALUES
    (
        '20000000-0000-4000-8000-000000000001',
        '10000000-0000-4000-8000-000000000001',
        '0101',
        'Nyx Voltage',
        'hunter',
        'avery@example.test',
        now() - interval '45 days'
    ),
    (
        '20000000-0000-4000-8000-000000000001',
        '10000000-0000-4000-8000-000000000002',
        '0202',
        'Rook Zero',
        'bounty',
        'jordan@example.test',
        now() - interval '20 days'
    ),
    (
        '20000000-0000-4000-8000-000000000001',
        '10000000-0000-4000-8000-000000000003',
        '0303',
        'Echo Vane',
        'unassigned',
        NULL,
        now() - interval '7 days'
    ),
    (
        '20000000-0000-4000-8000-000000000001',
        '10000000-0000-4000-8000-000000000004',
        '0404',
        'Chrome Wraith',
        'hunter',
        'sam@example.test',
        now() - interval '2 days'
    ),
    (
        '20000000-0000-4000-8000-000000000001',
        '10000000-0000-4000-8000-000000000005',
        '0505',
        'Nova Static',
        'bounty',
        'taylor@example.test',
        now() - interval '4 hours'
    )
ON CONFLICT (id) DO UPDATE SET
    event_id = EXCLUDED.event_id,
    pdn_code = EXCLUDED.pdn_code,
    name = EXCLUDED.name,
    mode = EXCLUDED.mode,
    email = EXCLUDED.email,
    created_at = EXCLUDED.created_at;

-- Never rewind the allocation counter when fixtures are reapplied.
UPDATE events
SET next_pdn_code = GREATEST(next_pdn_code, (
    SELECT COALESCE(MAX(pdn_code::integer), 0) + 1
    FROM players WHERE event_id = events.id
))
WHERE id = '20000000-0000-4000-8000-000000000001';

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
