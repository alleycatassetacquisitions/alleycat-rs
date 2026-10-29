INSERT INTO devices (id, id_label, hardware_version, mac_address) VALUES
    ('60000000-0000-4000-8000-000000000001', 1, 3, '02:00:00:00:01:01'),
    ('60000000-0000-4000-8000-000000000002', 2, NULL, NULL);
-- Preserve old failures, a later repair and a delayed upload, not just latest state.
INSERT INTO device_inspections (id, device_id, device_kind, inspected_at, firmware_commit_hash, notes) VALUES
    ('70000000-0000-4000-8000-000000000001', '60000000-0000-4000-8000-000000000001', 'pdn', '2026-10-01T12:00:00Z', 'abc123', 'Screen failed'),
    ('70000000-0000-4000-8000-000000000002', '60000000-0000-4000-8000-000000000001', 'pdn', '2026-10-03T12:00:00Z', 'def456', E'Repaired screen\nRetested ✓'),
    ('70000000-0000-4000-8000-000000000003', '60000000-0000-4000-8000-000000000001', 'pdn', '2026-10-02T12:00:00Z', NULL, 'Delayed upload');
INSERT INTO pdn_inspection_checks (inspection_id, screen, primary_button, housing) VALUES
    ('70000000-0000-4000-8000-000000000001', false, true, NULL),
    ('70000000-0000-4000-8000-000000000002', true, NULL, true),
    ('70000000-0000-4000-8000-000000000003', NULL, false, true);
INSERT INTO player_affiliations (player_id, event_id, category_id, affiliation_id)
VALUES ('10000000-0000-4000-8000-000000000001', '20000000-0000-4000-8000-000000000001', '40000000-0000-4000-8000-000000000001', '50000000-0000-4000-8000-000000000001');
INSERT INTO quickdraw_matches (
    id, event_id, player_a_id, player_b_id, player_a_team_id, player_b_team_id,
    player_a_draw_time_ms, player_b_draw_time_ms, winner_player_id, occurred_at, scoring_status
) VALUES (
    '80000000-0000-4000-8000-000000000001', '20000000-0000-4000-8000-000000000001',
    '10000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000002',
    '30000000-0000-4000-8000-000000000001', '30000000-0000-4000-8000-000000000002',
    100, 200, '10000000-0000-4000-8000-000000000001', '2026-10-01T12:00:00Z', 'scoring'
);
-- Unrelated future tables and sequence state must also survive without changing
-- the backup implementation or maintaining a list of application tables.
CREATE TABLE recovery_extra (id bigint GENERATED ALWAYS AS IDENTITY, value text);
INSERT INTO recovery_extra (value) VALUES ('preserve me'), ('deleted row advances sequence');
DELETE FROM recovery_extra WHERE id = 2;
