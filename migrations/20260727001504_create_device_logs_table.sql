create table device_logs (
    id uuid primary key default gen_random_uuid(),
    device_mac macaddr not null,
    crash_number bigint not null check (crash_number >= 0),
    uptime_ms bigint not null check (uptime_ms >= 0),
    received_at timestamptz not null default now(),
    software_version text,
    reset_reason integer not null check (reset_reason >= 0),
    program_counter bigint check (program_counter between 0 and 4294967295),
    exception_cause bigint check (exception_cause between 0 and 4294967295),
    task_name text,

    unique (device_mac, crash_number)
);
