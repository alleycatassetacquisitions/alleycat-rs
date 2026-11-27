CREATE TYPE public.event_uuid_range AS RANGE (
    subtype = uuid,
    multirange_type_name = public.event_uuid_multirange
);

CREATE TYPE public.quickdraw_scoring_status AS ENUM (
    'scoring',
    'same_team',
    'scoring_disabled',
    'unassigned_team',
    'delayed_scores_disabled',
    'tie'
);

CREATE TABLE public.events (
    id uuid NOT NULL,
    name text NOT NULL,
    venue_name text,
    next_pdn_code integer DEFAULT 1 NOT NULL,
    created_at timestamp with time zone NOT NULL,
    accepting_delayed_scores boolean DEFAULT true NOT NULL,
    CONSTRAINT events_next_pdn_code_check CHECK (((next_pdn_code >= 1) AND (next_pdn_code <= 10000))),
    CONSTRAINT events_pkey PRIMARY KEY (id)
);

CREATE TABLE public.devices (
    id uuid NOT NULL,
    id_label integer NOT NULL,
    device_kind text DEFAULT 'pdn'::text NOT NULL,
    hardware_version integer,
    mac_address macaddr,
    created_at timestamp with time zone DEFAULT clock_timestamp() NOT NULL,
    CONSTRAINT devices_device_kind_check CHECK ((device_kind = 'pdn'::text)),
    CONSTRAINT devices_hardware_version_check CHECK ((hardware_version > 0)),
    CONSTRAINT devices_id_check CHECK ((id <> '00000000-0000-0000-0000-000000000000'::uuid)),
    CONSTRAINT devices_id_label_check CHECK (((id_label >= 1) AND (id_label <= 9999))),
    CONSTRAINT devices_id_device_kind_key UNIQUE (id, device_kind),
    CONSTRAINT devices_id_label_key UNIQUE (id_label),
    CONSTRAINT devices_mac_address_key UNIQUE (mac_address),
    CONSTRAINT devices_pkey PRIMARY KEY (id)
);

CREATE TABLE public.device_logs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    device_mac macaddr NOT NULL,
    crash_number bigint NOT NULL,
    uptime_ms bigint NOT NULL,
    received_at timestamp with time zone DEFAULT now() NOT NULL,
    software_version text,
    reset_reason integer NOT NULL,
    program_counter bigint,
    exception_cause bigint,
    task_name text,
    CONSTRAINT device_logs_crash_number_check CHECK ((crash_number >= 0)),
    CONSTRAINT device_logs_exception_cause_check CHECK (((exception_cause >= 0) AND (exception_cause <= '4294967295'::bigint))),
    CONSTRAINT device_logs_program_counter_check CHECK (((program_counter >= 0) AND (program_counter <= '4294967295'::bigint))),
    CONSTRAINT device_logs_reset_reason_check CHECK ((reset_reason >= 0)),
    CONSTRAINT device_logs_uptime_ms_check CHECK ((uptime_ms >= 0)),
    CONSTRAINT device_logs_device_mac_crash_number_key UNIQUE (device_mac, crash_number),
    CONSTRAINT device_logs_pkey PRIMARY KEY (id)
);

CREATE TABLE public.reserved_pdn_codes (
    code text NOT NULL,
    reason text,
    CONSTRAINT reserved_pdn_codes_code_check CHECK ((code ~ '^[0-9]{4}$'::text)),
    CONSTRAINT reserved_pdn_codes_pkey PRIMARY KEY (code)
);

CREATE TABLE public.affiliation_categories (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    event_id uuid NOT NULL,
    name text NOT NULL,
    send_to_devices boolean DEFAULT false NOT NULL,
    CONSTRAINT affiliation_categories_name_check CHECK (((name <> ''::text) AND (name = btrim(name)) AND (char_length(name) <= 256))),
    CONSTRAINT affiliation_categories_event_id_key UNIQUE (event_id, id),
    CONSTRAINT affiliation_categories_event_name_key UNIQUE (event_id, name),
    CONSTRAINT affiliation_categories_pkey PRIMARY KEY (id),
    CONSTRAINT affiliation_categories_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id)
);

CREATE TABLE public.app_state (
    id integer NOT NULL,
    active_event_id uuid,
    CONSTRAINT app_state_id_check CHECK ((id = 1)),
    CONSTRAINT app_state_pkey PRIMARY KEY (id),
    CONSTRAINT app_state_active_event_id_fkey FOREIGN KEY (active_event_id) REFERENCES public.events(id)
);

CREATE TABLE public.event_scoring_periods (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    event_id uuid NOT NULL,
    started_at timestamp with time zone DEFAULT clock_timestamp() NOT NULL,
    closes_at timestamp with time zone,
    CONSTRAINT event_scoring_periods_finite_times CHECK ((isfinite(started_at) AND ((closes_at IS NULL) OR isfinite(closes_at)))),
    CONSTRAINT event_scoring_periods_time_order CHECK ((closes_at > started_at)),
    -- A single-UUID range lets GiST check for overlapping periods within each event.
    CONSTRAINT event_scoring_periods_no_overlap EXCLUDE USING gist (public.event_uuid_range(event_id, event_id, '[]'::text) WITH &&, tstzrange(started_at, closes_at, '[)'::text) WITH &&),
    CONSTRAINT event_scoring_periods_pkey PRIMARY KEY (id),
    CONSTRAINT event_scoring_periods_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id)
);

CREATE INDEX event_scoring_periods_event_id_idx ON public.event_scoring_periods USING btree (event_id);

CREATE TABLE public.quickdraw_matches_raw (
    id uuid NOT NULL,
    event_id uuid NOT NULL,
    match_id uuid,
    mac_address text,
    received_at timestamp with time zone DEFAULT clock_timestamp() NOT NULL,
    submitted_match_id text NOT NULL,
    player_a_id text NOT NULL,
    player_b_id text NOT NULL,
    player_a_team_id text,
    player_b_team_id text,
    player_a_draw_time_ms numeric(20,0),
    player_b_draw_time_ms numeric(20,0),
    winner_player_id text,
    occurred_at_unix_ms bigint,
    failure_reason text,
    submitted_mac_address text,
    outcome text,
    comparison_key text,
    CONSTRAINT quickdraw_matches_raw_outcome_check CHECK ((outcome = ANY (ARRAY['completed'::text, 'tie'::text, 'failed'::text]))),
    CONSTRAINT quickdraw_matches_raw_player_a_draw_time_ms_check CHECK (((player_a_draw_time_ms >= (0)::numeric) AND (player_a_draw_time_ms <= '18446744073709551615'::numeric))),
    CONSTRAINT quickdraw_matches_raw_player_b_draw_time_ms_check CHECK (((player_b_draw_time_ms >= (0)::numeric) AND (player_b_draw_time_ms <= '18446744073709551615'::numeric))),
    CONSTRAINT quickdraw_matches_raw_pkey PRIMARY KEY (id),
    CONSTRAINT quickdraw_matches_raw_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id)
);

CREATE INDEX quickdraw_matches_raw_event_match_idx ON public.quickdraw_matches_raw USING btree (event_id, match_id);

CREATE INDEX quickdraw_matches_raw_event_received_idx ON public.quickdraw_matches_raw USING btree (event_id, received_at DESC, id DESC);

CREATE TABLE public.teams (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    event_id uuid NOT NULL,
    name text NOT NULL,
    CONSTRAINT teams_name_check CHECK (((name <> ''::text) AND (name = btrim(name)))),
    CONSTRAINT teams_event_id_key UNIQUE (event_id, id),
    CONSTRAINT teams_event_name_key UNIQUE (event_id, name),
    CONSTRAINT teams_pkey PRIMARY KEY (id),
    CONSTRAINT teams_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id)
);

CREATE TABLE public.device_inspections (
    id uuid NOT NULL,
    device_id uuid NOT NULL,
    device_kind text NOT NULL,
    inspected_at timestamp with time zone NOT NULL,
    received_at timestamp with time zone DEFAULT clock_timestamp() NOT NULL,
    firmware_commit_hash text,
    notes text,
    powers_on boolean,
    CONSTRAINT device_inspections_device_kind_check CHECK ((device_kind = 'pdn'::text)),
    CONSTRAINT device_inspections_firmware_commit_hash_check CHECK ((((octet_length(firmware_commit_hash) >= 1) AND (octet_length(firmware_commit_hash) <= 64)) AND (btrim(firmware_commit_hash) <> ''::text))),
    CONSTRAINT device_inspections_id_check CHECK ((id <> '00000000-0000-0000-0000-000000000000'::uuid)),
    CONSTRAINT device_inspections_inspected_at_check CHECK (((inspected_at >= '1970-01-01 00:00:00+00'::timestamp with time zone) AND (inspected_at <= '9999-12-31 23:59:59.999+00'::timestamp with time zone))),
    CONSTRAINT device_inspections_notes_check CHECK ((octet_length(notes) <= 2048)),
    CONSTRAINT device_inspections_id_device_kind_key UNIQUE (id, device_kind),
    CONSTRAINT device_inspections_pkey PRIMARY KEY (id),
    CONSTRAINT device_inspections_device_id_device_kind_fkey FOREIGN KEY (device_id, device_kind) REFERENCES public.devices(id, device_kind)
);

CREATE INDEX device_inspections_latest_idx ON public.device_inspections USING btree (device_id, inspected_at DESC, received_at DESC, id DESC);

CREATE TABLE public.affiliations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    event_id uuid NOT NULL,
    category_id uuid NOT NULL,
    name text NOT NULL,
    CONSTRAINT affiliations_name_check CHECK (((name <> ''::text) AND (name = btrim(name)) AND (char_length(name) <= 256))),
    CONSTRAINT affiliations_category_name_key UNIQUE (category_id, name),
    CONSTRAINT affiliations_event_category_id_key UNIQUE (event_id, category_id, id),
    CONSTRAINT affiliations_pkey PRIMARY KEY (id),
    CONSTRAINT affiliations_category_fkey FOREIGN KEY (event_id, category_id) REFERENCES public.affiliation_categories(event_id, id)
);

CREATE TABLE public.players (
    id uuid NOT NULL,
    pdn_code text NOT NULL,
    name text NOT NULL,
    email text,
    created_at timestamp with time zone NOT NULL,
    event_id uuid NOT NULL,
    team_id uuid,
    active boolean DEFAULT true NOT NULL,
    CONSTRAINT players_pdn_code_check CHECK ((pdn_code ~ '^[0-9]{4}$'::text)),
    CONSTRAINT players_event_id_key UNIQUE (event_id, id),
    CONSTRAINT players_event_name_key UNIQUE (event_id, name),
    CONSTRAINT players_event_pdn_code_key UNIQUE (event_id, pdn_code),
    CONSTRAINT players_pkey PRIMARY KEY (id),
    CONSTRAINT players_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id),
    CONSTRAINT players_event_team_fkey FOREIGN KEY (event_id, team_id) REFERENCES public.teams(event_id, id)
);

CREATE INDEX players_event_created_at_id_idx ON public.players USING btree (event_id, created_at DESC, id DESC);

CREATE TABLE public.pdn_inspection_checks (
    inspection_id uuid NOT NULL,
    device_kind text DEFAULT 'pdn'::text NOT NULL,
    screen boolean,
    primary_button boolean,
    secondary_button boolean,
    haptics boolean,
    leds boolean,
    power_switch boolean,
    input_jack boolean,
    output_jack boolean,
    housing boolean,
    CONSTRAINT pdn_inspection_checks_device_kind_check CHECK ((device_kind = 'pdn'::text)),
    CONSTRAINT pdn_inspection_checks_pkey PRIMARY KEY (inspection_id),
    CONSTRAINT pdn_inspection_checks_inspection_id_device_kind_fkey FOREIGN KEY (inspection_id, device_kind) REFERENCES public.device_inspections(id, device_kind)
);

CREATE TABLE public.player_affiliations (
    event_id uuid NOT NULL,
    player_id uuid NOT NULL,
    category_id uuid NOT NULL,
    affiliation_id uuid NOT NULL,
    CONSTRAINT player_affiliations_pkey PRIMARY KEY (player_id, category_id),
    CONSTRAINT player_affiliations_affiliation_fkey FOREIGN KEY (event_id, category_id, affiliation_id) REFERENCES public.affiliations(event_id, category_id, id),
    CONSTRAINT player_affiliations_player_fkey FOREIGN KEY (event_id, player_id) REFERENCES public.players(event_id, id)
);

CREATE INDEX player_affiliations_affiliation_idx ON public.player_affiliations USING btree (event_id, category_id, affiliation_id);

CREATE TABLE public.quickdraw_matches (
    id uuid NOT NULL,
    event_id uuid NOT NULL,
    player_a_id uuid NOT NULL,
    player_b_id uuid NOT NULL,
    player_a_team_id uuid,
    player_b_team_id uuid,
    player_a_draw_time_ms bigint NOT NULL,
    player_b_draw_time_ms bigint NOT NULL,
    winner_player_id uuid,
    occurred_at timestamp with time zone NOT NULL,
    received_at timestamp with time zone DEFAULT clock_timestamp() NOT NULL,
    scoring_status public.quickdraw_scoring_status NOT NULL,
    CONSTRAINT quickdraw_matches_distinct_players CHECK ((player_a_id <> player_b_id)),
    CONSTRAINT quickdraw_matches_occurred_at_check CHECK (isfinite(occurred_at)),
    CONSTRAINT quickdraw_matches_player_a_draw_time_ms_check CHECK ((player_a_draw_time_ms >= 0)),
    CONSTRAINT quickdraw_matches_player_b_draw_time_ms_check CHECK ((player_b_draw_time_ms >= 0)),
    CONSTRAINT quickdraw_matches_received_at_check CHECK (isfinite(received_at)),
    CONSTRAINT quickdraw_matches_scoring_teams_check CHECK ((((winner_player_id IS NULL) AND ((scoring_status)::text = 'tie'::text)) OR ((winner_player_id IS NOT NULL) AND (((scoring_status)::text = ANY (ARRAY['scoring_disabled'::text, 'delayed_scores_disabled'::text])) OR (scoring_status =
CASE
    WHEN ((player_a_team_id IS NULL) OR (player_b_team_id IS NULL)) THEN 'unassigned_team'::public.quickdraw_scoring_status
    WHEN (player_a_team_id = player_b_team_id) THEN 'same_team'::public.quickdraw_scoring_status
    ELSE 'scoring'::public.quickdraw_scoring_status
END))))),
    CONSTRAINT quickdraw_matches_winner_is_participant CHECK (((winner_player_id = player_a_id) OR (winner_player_id = player_b_id))),
    CONSTRAINT quickdraw_matches_pkey PRIMARY KEY (id),
    CONSTRAINT quickdraw_matches_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id),
    CONSTRAINT quickdraw_matches_player_a_fkey FOREIGN KEY (event_id, player_a_id) REFERENCES public.players(event_id, id),
    CONSTRAINT quickdraw_matches_player_b_fkey FOREIGN KEY (event_id, player_b_id) REFERENCES public.players(event_id, id),
    CONSTRAINT quickdraw_matches_team_a_fkey FOREIGN KEY (event_id, player_a_team_id) REFERENCES public.teams(event_id, id),
    CONSTRAINT quickdraw_matches_team_b_fkey FOREIGN KEY (event_id, player_b_team_id) REFERENCES public.teams(event_id, id)
);

CREATE INDEX quickdraw_matches_event_occurred_at_id_idx ON public.quickdraw_matches USING btree (event_id, occurred_at DESC, id DESC);

CREATE INDEX quickdraw_matches_player_a_history_idx ON public.quickdraw_matches USING btree (event_id, player_a_id, occurred_at, id);

CREATE INDEX quickdraw_matches_player_b_history_idx ON public.quickdraw_matches USING btree (event_id, player_b_id, occurred_at, id);
