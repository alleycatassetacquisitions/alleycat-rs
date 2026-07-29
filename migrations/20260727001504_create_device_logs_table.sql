create table device_logs (
    id uuid primary key,
    device_id uuid not null,
    report_id uuid not null,
    occurred_at timestamptz,
    received_at timestamptz not null,
    software_version text,
    crash_type text,
    message text,
    backtrace text,
    context jsonb not null default '{}',
    unique (device_id, report_id)
);
