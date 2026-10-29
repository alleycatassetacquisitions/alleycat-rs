CREATE TABLE devices (
    id uuid PRIMARY KEY CHECK (id <> '00000000-0000-0000-0000-000000000000'),
    id_label integer NOT NULL UNIQUE CHECK (id_label BETWEEN 1 AND 9999),
    device_kind text NOT NULL DEFAULT 'pdn' CHECK (device_kind = 'pdn'),
    hardware_version integer CHECK (hardware_version > 0),
    mac_address macaddr UNIQUE,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE (id, device_kind)
);

CREATE TABLE device_inspections (
    id uuid PRIMARY KEY CHECK (id <> '00000000-0000-0000-0000-000000000000'),
    device_id uuid NOT NULL,
    device_kind text NOT NULL CHECK (device_kind = 'pdn'),
    inspected_at timestamptz NOT NULL CHECK (
        inspected_at >= '1970-01-01T00:00:00Z' AND inspected_at <= '9999-12-31T23:59:59.999Z'
    ),
    received_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    firmware_commit_hash text CHECK (octet_length(firmware_commit_hash) BETWEEN 1 AND 64 AND btrim(firmware_commit_hash) <> ''),
    notes text CHECK (octet_length(notes) <= 2048),
    UNIQUE (id, device_kind),
    FOREIGN KEY (device_id, device_kind) REFERENCES devices(id, device_kind)
);

CREATE TABLE pdn_inspection_checks (
    inspection_id uuid PRIMARY KEY,
    device_kind text NOT NULL DEFAULT 'pdn' CHECK (device_kind = 'pdn'),
    FOREIGN KEY (inspection_id, device_kind) REFERENCES device_inspections(id, device_kind),
    screen boolean,
    primary_button boolean,
    secondary_button boolean,
    haptics boolean,
    leds boolean,
    power_switch boolean,
    input_jack boolean,
    output_jack boolean,
    housing boolean
);

CREATE INDEX device_inspections_latest_idx
    ON device_inspections (device_id, inspected_at DESC, received_at DESC, id DESC);

COMMENT ON TABLE devices IS
    'Physical device inventory shared across events. IDs and printed labels identify devices independently of inspection history. Currently supports PDNs.';
COMMENT ON COLUMN devices.mac_address IS
    'Optional 48-bit hardware address. Unique when known; not the permanent inventory identity.';
COMMENT ON TABLE device_inspections IS
    'Saved condition checks. Each row is a complete snapshot; missing checks are unknown, never inherited from earlier inspections. API writes never replace an accepted inspection.';
COMMENT ON COLUMN device_inspections.inspected_at IS
    'Client-reported check time in UTC. Latest condition uses this time, then received_at and ID descending to resolve ties.';
COMMENT ON COLUMN device_inspections.firmware_commit_hash IS
    'Firmware observed during this check, not a mutable device attribute. Null means not recorded.';
COMMENT ON TABLE pdn_inspection_checks IS
    'PDN component observations for one inspection. Shared identity, timestamps, firmware, and notes live in device_inspections. Other device classes can define their own checks.';
COMMENT ON COLUMN pdn_inspection_checks.screen IS 'True means working, false means failed, null means not checked during this inspection.';
COMMENT ON COLUMN pdn_inspection_checks.primary_button IS 'True means working, false means failed, null means not checked during this inspection.';
COMMENT ON COLUMN pdn_inspection_checks.secondary_button IS 'True means working, false means failed, null means not checked during this inspection.';
COMMENT ON COLUMN pdn_inspection_checks.haptics IS 'True means working, false means failed, null means not checked during this inspection.';
COMMENT ON COLUMN pdn_inspection_checks.leds IS 'True means working, false means failed, null means not checked during this inspection.';
COMMENT ON COLUMN pdn_inspection_checks.power_switch IS 'True means working, false means failed, null means not checked during this inspection.';
COMMENT ON COLUMN pdn_inspection_checks.input_jack IS 'True means working, false means failed, null means not checked during this inspection.';
COMMENT ON COLUMN pdn_inspection_checks.output_jack IS 'True means working, false means failed, null means not checked during this inspection.';
COMMENT ON COLUMN pdn_inspection_checks.housing IS 'True means working, false means failed, null means not checked during this inspection.';
