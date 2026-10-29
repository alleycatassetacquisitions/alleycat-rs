--
-- PostgreSQL database dump
--



SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: event_uuid_range; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.event_uuid_range AS RANGE (
    subtype = uuid,
    multirange_type_name = public.event_uuid_multirange
);


--
-- Name: TYPE event_uuid_range; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TYPE public.event_uuid_range IS 'Singleton event UUID ranges let the scoring-period exclusion constraint compare events using core GiST range operators, without btree_gist. Construct both bounds from the same UUID with inclusive bounds.';


--
-- Name: quickdraw_scoring_status; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.quickdraw_scoring_status AS ENUM (
    'scoring',
    'same_team',
    'scoring_disabled',
    'unassigned_team',
    'delayed_scores_disabled'
);


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: affiliation_categories; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.affiliation_categories (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    event_id uuid NOT NULL,
    name text NOT NULL,
    send_to_devices boolean DEFAULT false NOT NULL,
    CONSTRAINT affiliation_categories_name_check CHECK (((name <> ''::text) AND (name = btrim(name)) AND (char_length(name) <= 256)))
);


--
-- Name: TABLE affiliation_categories; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.affiliation_categories IS 'Operator-defined affiliation categories within an event. No seeded categories or implied gameplay behavior.';


--
-- Name: COLUMN affiliation_categories.send_to_devices; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.affiliation_categories.send_to_devices IS 'Include assignments in this category in device player lookups. Defaults to false; staff responses always include all assignments.';


--
-- Name: affiliations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.affiliations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    event_id uuid NOT NULL,
    category_id uuid NOT NULL,
    name text NOT NULL,
    CONSTRAINT affiliations_name_check CHECK (((name <> ''::text) AND (name = btrim(name)) AND (char_length(name) <= 256)))
);


--
-- Name: TABLE affiliations; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.affiliations IS 'Operator-defined affiliations within a category. Names have no implicit gameplay or permission behavior.';


--
-- Name: app_state; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.app_state (
    id integer NOT NULL,
    active_event_id uuid,
    CONSTRAINT app_state_id_check CHECK ((id = 1))
);


--
-- Name: device_inspections; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.device_inspections (
    id uuid NOT NULL,
    device_id uuid NOT NULL,
    device_kind text NOT NULL,
    inspected_at timestamp with time zone NOT NULL,
    received_at timestamp with time zone DEFAULT clock_timestamp() NOT NULL,
    firmware_commit_hash text,
    notes text,
    CONSTRAINT device_inspections_device_kind_check CHECK ((device_kind = 'pdn'::text)),
    CONSTRAINT device_inspections_firmware_commit_hash_check CHECK ((((octet_length(firmware_commit_hash) >= 1) AND (octet_length(firmware_commit_hash) <= 64)) AND (btrim(firmware_commit_hash) <> ''::text))),
    CONSTRAINT device_inspections_id_check CHECK ((id <> '00000000-0000-0000-0000-000000000000'::uuid)),
    CONSTRAINT device_inspections_inspected_at_check CHECK (((inspected_at >= '1970-01-01 00:00:00+00'::timestamp with time zone) AND (inspected_at <= '9999-12-31 23:59:59.999+00'::timestamp with time zone))),
    CONSTRAINT device_inspections_notes_check CHECK ((octet_length(notes) <= 2048))
);


--
-- Name: TABLE device_inspections; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.device_inspections IS 'Saved condition checks. Each row is a complete snapshot; missing checks are unknown, never inherited from earlier inspections. API writes never replace an accepted inspection.';


--
-- Name: COLUMN device_inspections.inspected_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.device_inspections.inspected_at IS 'Client-reported check time in UTC. Latest condition uses this time, then received_at and ID descending to resolve ties.';


--
-- Name: COLUMN device_inspections.firmware_commit_hash; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.device_inspections.firmware_commit_hash IS 'Firmware observed during this check, not a mutable device attribute. Null means not recorded.';


--
-- Name: device_logs; Type: TABLE; Schema: public; Owner: -
--

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
    CONSTRAINT device_logs_uptime_ms_check CHECK ((uptime_ms >= 0))
);


--
-- Name: devices; Type: TABLE; Schema: public; Owner: -
--

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
    CONSTRAINT devices_id_label_check CHECK (((id_label >= 1) AND (id_label <= 9999)))
);


--
-- Name: TABLE devices; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.devices IS 'Physical device inventory shared across events. IDs and printed labels identify devices independently of inspection history. Currently supports PDNs.';


--
-- Name: COLUMN devices.mac_address; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.devices.mac_address IS 'Optional 48-bit hardware address. Unique when known; not the permanent inventory identity.';


--
-- Name: event_scoring_periods; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.event_scoring_periods (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    event_id uuid NOT NULL,
    started_at timestamp with time zone DEFAULT clock_timestamp() NOT NULL,
    closes_at timestamp with time zone,
    CONSTRAINT event_scoring_periods_finite_times CHECK ((isfinite(started_at) AND ((closes_at IS NULL) OR isfinite(closes_at)))),
    CONSTRAINT event_scoring_periods_time_order CHECK ((closes_at > started_at))
);


--
-- Name: TABLE event_scoring_periods; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.event_scoring_periods IS 'Event scoring history. Opening creates a row; closing fills closes_at. Match eligibility uses device occurrence time, not upload time. No unclosed period means scoring is disabled.';


--
-- Name: COLUMN event_scoring_periods.started_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.event_scoring_periods.started_at IS 'Inclusive scoring start instant. Stored as timestamptz; devices report UTC.';


--
-- Name: COLUMN event_scoring_periods.closes_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.event_scoring_periods.closes_at IS 'Exclusive scoring end instant. NULL means this period is still open. Closing a period preserves its history.';


--
-- Name: events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.events (
    id uuid NOT NULL,
    name text NOT NULL,
    venue_name text,
    next_pdn_code integer DEFAULT 1 NOT NULL,
    created_at timestamp with time zone NOT NULL,
    accepting_delayed_scores boolean DEFAULT true NOT NULL,
    CONSTRAINT events_next_pdn_code_check CHECK (((next_pdn_code >= 1) AND (next_pdn_code <= 10000)))
);


--
-- Name: COLUMN events.accepting_delayed_scores; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.events.accepting_delayed_scores IS 'Reversible per-event switch, default true. When false, new matches from closed scoring periods are recorded as delayed_scores_disabled. Open-period scoring and previously accepted results are unchanged.';


--
-- Name: pdn_inspection_checks; Type: TABLE; Schema: public; Owner: -
--

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
    CONSTRAINT pdn_inspection_checks_device_kind_check CHECK ((device_kind = 'pdn'::text))
);


--
-- Name: TABLE pdn_inspection_checks; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.pdn_inspection_checks IS 'PDN component observations for one inspection. Shared identity, timestamps, firmware, and notes live in device_inspections. Other device classes can define their own checks.';


--
-- Name: COLUMN pdn_inspection_checks.screen; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.pdn_inspection_checks.screen IS 'True means working, false means failed, null means not checked during this inspection.';


--
-- Name: COLUMN pdn_inspection_checks.primary_button; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.pdn_inspection_checks.primary_button IS 'True means working, false means failed, null means not checked during this inspection.';


--
-- Name: COLUMN pdn_inspection_checks.secondary_button; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.pdn_inspection_checks.secondary_button IS 'True means working, false means failed, null means not checked during this inspection.';


--
-- Name: COLUMN pdn_inspection_checks.haptics; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.pdn_inspection_checks.haptics IS 'True means working, false means failed, null means not checked during this inspection.';


--
-- Name: COLUMN pdn_inspection_checks.leds; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.pdn_inspection_checks.leds IS 'True means working, false means failed, null means not checked during this inspection.';


--
-- Name: COLUMN pdn_inspection_checks.power_switch; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.pdn_inspection_checks.power_switch IS 'True means working, false means failed, null means not checked during this inspection.';


--
-- Name: COLUMN pdn_inspection_checks.input_jack; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.pdn_inspection_checks.input_jack IS 'True means working, false means failed, null means not checked during this inspection.';


--
-- Name: COLUMN pdn_inspection_checks.output_jack; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.pdn_inspection_checks.output_jack IS 'True means working, false means failed, null means not checked during this inspection.';


--
-- Name: COLUMN pdn_inspection_checks.housing; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.pdn_inspection_checks.housing IS 'True means working, false means failed, null means not checked during this inspection.';


--
-- Name: player_affiliations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.player_affiliations (
    event_id uuid NOT NULL,
    player_id uuid NOT NULL,
    category_id uuid NOT NULL,
    affiliation_id uuid NOT NULL
);


--
-- Name: TABLE player_affiliations; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.player_affiliations IS 'At most one affiliation per player per category. Composite foreign keys keep player, category, and affiliation in the same event. Clearing an assignment deletes its row; changes do not affect scores.';


--
-- Name: players; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.players (
    id uuid NOT NULL,
    pdn_code text NOT NULL,
    name text NOT NULL,
    email text,
    created_at timestamp with time zone NOT NULL,
    event_id uuid NOT NULL,
    team_id uuid,
    active boolean DEFAULT true NOT NULL,
    CONSTRAINT players_pdn_code_check CHECK ((pdn_code ~ '^[0-9]{4}$'::text))
);


--
-- Name: COLUMN players.active; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.players.active IS 'Reversible registration/device status, initially true. Inactive players retain their PIN, name, event, and match history and remain discoverable. This flag does not change accepted scores or reject delayed match uploads.';


--
-- Name: quickdraw_matches; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.quickdraw_matches (
    id uuid NOT NULL,
    event_id uuid NOT NULL,
    player_a_id uuid NOT NULL,
    player_b_id uuid NOT NULL,
    player_a_team_id uuid,
    player_b_team_id uuid,
    player_a_draw_time_ms bigint NOT NULL,
    player_b_draw_time_ms bigint NOT NULL,
    winner_player_id uuid NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    received_at timestamp with time zone DEFAULT clock_timestamp() NOT NULL,
    scoring_status public.quickdraw_scoring_status NOT NULL,
    CONSTRAINT quickdraw_matches_distinct_players CHECK ((player_a_id <> player_b_id)),
    CONSTRAINT quickdraw_matches_occurred_at_check CHECK (isfinite(occurred_at)),
    CONSTRAINT quickdraw_matches_player_a_draw_time_ms_check CHECK ((player_a_draw_time_ms >= 0)),
    CONSTRAINT quickdraw_matches_player_b_draw_time_ms_check CHECK ((player_b_draw_time_ms >= 0)),
    CONSTRAINT quickdraw_matches_received_at_check CHECK (isfinite(received_at)),
    CONSTRAINT quickdraw_matches_scoring_teams_check CHECK ((((scoring_status)::text = ANY (ARRAY['scoring_disabled'::text, 'delayed_scores_disabled'::text])) OR (scoring_status =
CASE
    WHEN ((player_a_team_id IS NULL) OR (player_b_team_id IS NULL)) THEN 'unassigned_team'::public.quickdraw_scoring_status
    WHEN (player_a_team_id = player_b_team_id) THEN 'same_team'::public.quickdraw_scoring_status
    ELSE 'scoring'::public.quickdraw_scoring_status
END))),
    CONSTRAINT quickdraw_matches_winner_is_participant CHECK (((winner_player_id = player_a_id) OR (winner_player_id = player_b_id)))
);


--
-- Name: TABLE quickdraw_matches; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.quickdraw_matches IS 'Completed two-player quickdraw results, including non-scoring duels. Ingestion must use INSERT ... ON CONFLICT (id) DO NOTHING: the first accepted report and its device timestamp win. No per-device reports or leaderboard totals are stored.';


--
-- Name: COLUMN quickdraw_matches.player_a_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.quickdraw_matches.player_a_id IS 'First participant slot, not a Hunter/Bounty role. Device reports must identify the same shared duel ID.';


--
-- Name: COLUMN quickdraw_matches.player_a_team_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.quickdraw_matches.player_a_team_id IS 'Team credited at match time, supplied by ingestion; NULL means unassigned. Historical snapshot, not a reference to current player membership.';


--
-- Name: COLUMN quickdraw_matches.player_b_team_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.quickdraw_matches.player_b_team_id IS 'Team credited at match time, supplied by ingestion; NULL means unassigned. Historical snapshot, not a reference to current player membership.';


--
-- Name: COLUMN quickdraw_matches.player_a_draw_time_ms; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.quickdraw_matches.player_a_draw_time_ms IS 'Device-measured draw duration in milliseconds. Nonnegative; zero is a measurement, not a missing-value sentinel.';


--
-- Name: COLUMN quickdraw_matches.player_b_draw_time_ms; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.quickdraw_matches.player_b_draw_time_ms IS 'Device-measured draw duration in milliseconds. Nonnegative; zero is a measurement, not a missing-value sentinel.';


--
-- Name: COLUMN quickdraw_matches.winner_player_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.quickdraw_matches.winner_player_id IS 'Explicit device-reported winner; must be one of the two participants. No inferred winner from draw times.';


--
-- Name: COLUMN quickdraw_matches.occurred_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.quickdraw_matches.occurred_at IS 'Trusted UTC device timestamp from the first accepted upload. Use for scoring periods, daily boundaries and match chronology.';


--
-- Name: COLUMN quickdraw_matches.received_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.quickdraw_matches.received_at IS 'Defaults to the database clock at insertion. Ingestion must omit this field and preserve the first accepted result on duplicate uploads. Not the time used for scoring eligibility.';


--
-- Name: COLUMN quickdraw_matches.scoring_status; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.quickdraw_matches.scoring_status IS 'First accepted scoring decision: outside a scoring period -> scoring_disabled; closed period with delayed scores disabled -> delayed_scores_disabled; either team missing -> unassigned_team; equal teams -> same_team; otherwise scoring. Ingestion checks period history and the event switch under the event row lock. The database checks team consistency; duplicates never rescore.';


--
-- Name: reserved_pdn_codes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.reserved_pdn_codes (
    code text NOT NULL,
    reason text,
    CONSTRAINT reserved_pdn_codes_code_check CHECK ((code ~ '^[0-9]{4}$'::text))
);


--
-- Name: teams; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.teams (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    event_id uuid NOT NULL,
    name text NOT NULL,
    CONSTRAINT teams_name_check CHECK (((name <> ''::text) AND (name = btrim(name))))
);


--
-- Name: affiliation_categories affiliation_categories_event_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.affiliation_categories
    ADD CONSTRAINT affiliation_categories_event_id_key UNIQUE (event_id, id);


--
-- Name: affiliation_categories affiliation_categories_event_name_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.affiliation_categories
    ADD CONSTRAINT affiliation_categories_event_name_key UNIQUE (event_id, name);


--
-- Name: affiliation_categories affiliation_categories_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.affiliation_categories
    ADD CONSTRAINT affiliation_categories_pkey PRIMARY KEY (id);


--
-- Name: affiliations affiliations_category_name_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.affiliations
    ADD CONSTRAINT affiliations_category_name_key UNIQUE (category_id, name);


--
-- Name: affiliations affiliations_event_category_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.affiliations
    ADD CONSTRAINT affiliations_event_category_id_key UNIQUE (event_id, category_id, id);


--
-- Name: affiliations affiliations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.affiliations
    ADD CONSTRAINT affiliations_pkey PRIMARY KEY (id);


--
-- Name: app_state app_state_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.app_state
    ADD CONSTRAINT app_state_pkey PRIMARY KEY (id);


--
-- Name: device_inspections device_inspections_id_device_kind_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_inspections
    ADD CONSTRAINT device_inspections_id_device_kind_key UNIQUE (id, device_kind);


--
-- Name: device_inspections device_inspections_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_inspections
    ADD CONSTRAINT device_inspections_pkey PRIMARY KEY (id);


--
-- Name: device_logs device_logs_device_mac_crash_number_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_logs
    ADD CONSTRAINT device_logs_device_mac_crash_number_key UNIQUE (device_mac, crash_number);


--
-- Name: device_logs device_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_logs
    ADD CONSTRAINT device_logs_pkey PRIMARY KEY (id);


--
-- Name: devices devices_id_device_kind_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_id_device_kind_key UNIQUE (id, device_kind);


--
-- Name: devices devices_id_label_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_id_label_key UNIQUE (id_label);


--
-- Name: devices devices_mac_address_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_mac_address_key UNIQUE (mac_address);


--
-- Name: devices devices_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_pkey PRIMARY KEY (id);


--
-- Name: event_scoring_periods event_scoring_periods_no_overlap; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_scoring_periods
    ADD CONSTRAINT event_scoring_periods_no_overlap EXCLUDE USING gist (public.event_uuid_range(event_id, event_id, '[]'::text) WITH &&, tstzrange(started_at, closes_at, '[)'::text) WITH &&);


--
-- Name: event_scoring_periods event_scoring_periods_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_scoring_periods
    ADD CONSTRAINT event_scoring_periods_pkey PRIMARY KEY (id);


--
-- Name: events events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.events
    ADD CONSTRAINT events_pkey PRIMARY KEY (id);


--
-- Name: pdn_inspection_checks pdn_inspection_checks_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.pdn_inspection_checks
    ADD CONSTRAINT pdn_inspection_checks_pkey PRIMARY KEY (inspection_id);


--
-- Name: player_affiliations player_affiliations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.player_affiliations
    ADD CONSTRAINT player_affiliations_pkey PRIMARY KEY (player_id, category_id);


--
-- Name: players players_event_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.players
    ADD CONSTRAINT players_event_id_key UNIQUE (event_id, id);


--
-- Name: players players_event_name_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.players
    ADD CONSTRAINT players_event_name_key UNIQUE (event_id, name);


--
-- Name: players players_event_pdn_code_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.players
    ADD CONSTRAINT players_event_pdn_code_key UNIQUE (event_id, pdn_code);


--
-- Name: players players_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.players
    ADD CONSTRAINT players_pkey PRIMARY KEY (id);


--
-- Name: quickdraw_matches quickdraw_matches_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.quickdraw_matches
    ADD CONSTRAINT quickdraw_matches_pkey PRIMARY KEY (id);


--
-- Name: reserved_pdn_codes reserved_pdn_codes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reserved_pdn_codes
    ADD CONSTRAINT reserved_pdn_codes_pkey PRIMARY KEY (code);


--
-- Name: teams teams_event_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.teams
    ADD CONSTRAINT teams_event_id_key UNIQUE (event_id, id);


--
-- Name: teams teams_event_name_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.teams
    ADD CONSTRAINT teams_event_name_key UNIQUE (event_id, name);


--
-- Name: teams teams_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.teams
    ADD CONSTRAINT teams_pkey PRIMARY KEY (id);


--
-- Name: device_inspections_latest_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX device_inspections_latest_idx ON public.device_inspections USING btree (device_id, inspected_at DESC, received_at DESC, id DESC);


--
-- Name: event_scoring_periods_event_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX event_scoring_periods_event_id_idx ON public.event_scoring_periods USING btree (event_id);


--
-- Name: player_affiliations_affiliation_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX player_affiliations_affiliation_idx ON public.player_affiliations USING btree (event_id, category_id, affiliation_id);


--
-- Name: players_event_created_at_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX players_event_created_at_id_idx ON public.players USING btree (event_id, created_at DESC, id DESC);


--
-- Name: quickdraw_matches_event_occurred_at_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX quickdraw_matches_event_occurred_at_id_idx ON public.quickdraw_matches USING btree (event_id, occurred_at DESC, id DESC);


--
-- Name: quickdraw_matches_player_a_history_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX quickdraw_matches_player_a_history_idx ON public.quickdraw_matches USING btree (event_id, player_a_id, occurred_at, id);


--
-- Name: quickdraw_matches_player_b_history_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX quickdraw_matches_player_b_history_idx ON public.quickdraw_matches USING btree (event_id, player_b_id, occurred_at, id);


--
-- Name: affiliation_categories affiliation_categories_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.affiliation_categories
    ADD CONSTRAINT affiliation_categories_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id);


--
-- Name: affiliations affiliations_category_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.affiliations
    ADD CONSTRAINT affiliations_category_fkey FOREIGN KEY (event_id, category_id) REFERENCES public.affiliation_categories(event_id, id);


--
-- Name: app_state app_state_active_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.app_state
    ADD CONSTRAINT app_state_active_event_id_fkey FOREIGN KEY (active_event_id) REFERENCES public.events(id);


--
-- Name: device_inspections device_inspections_device_id_device_kind_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_inspections
    ADD CONSTRAINT device_inspections_device_id_device_kind_fkey FOREIGN KEY (device_id, device_kind) REFERENCES public.devices(id, device_kind);


--
-- Name: event_scoring_periods event_scoring_periods_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_scoring_periods
    ADD CONSTRAINT event_scoring_periods_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id);


--
-- Name: pdn_inspection_checks pdn_inspection_checks_inspection_id_device_kind_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.pdn_inspection_checks
    ADD CONSTRAINT pdn_inspection_checks_inspection_id_device_kind_fkey FOREIGN KEY (inspection_id, device_kind) REFERENCES public.device_inspections(id, device_kind);


--
-- Name: player_affiliations player_affiliations_affiliation_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.player_affiliations
    ADD CONSTRAINT player_affiliations_affiliation_fkey FOREIGN KEY (event_id, category_id, affiliation_id) REFERENCES public.affiliations(event_id, category_id, id);


--
-- Name: player_affiliations player_affiliations_player_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.player_affiliations
    ADD CONSTRAINT player_affiliations_player_fkey FOREIGN KEY (event_id, player_id) REFERENCES public.players(event_id, id);


--
-- Name: players players_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.players
    ADD CONSTRAINT players_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id);


--
-- Name: players players_event_team_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.players
    ADD CONSTRAINT players_event_team_fkey FOREIGN KEY (event_id, team_id) REFERENCES public.teams(event_id, id);


--
-- Name: quickdraw_matches quickdraw_matches_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.quickdraw_matches
    ADD CONSTRAINT quickdraw_matches_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id);


--
-- Name: quickdraw_matches quickdraw_matches_player_a_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.quickdraw_matches
    ADD CONSTRAINT quickdraw_matches_player_a_fkey FOREIGN KEY (event_id, player_a_id) REFERENCES public.players(event_id, id);


--
-- Name: quickdraw_matches quickdraw_matches_player_b_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.quickdraw_matches
    ADD CONSTRAINT quickdraw_matches_player_b_fkey FOREIGN KEY (event_id, player_b_id) REFERENCES public.players(event_id, id);


--
-- Name: quickdraw_matches quickdraw_matches_team_a_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.quickdraw_matches
    ADD CONSTRAINT quickdraw_matches_team_a_fkey FOREIGN KEY (event_id, player_a_team_id) REFERENCES public.teams(event_id, id);


--
-- Name: quickdraw_matches quickdraw_matches_team_b_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.quickdraw_matches
    ADD CONSTRAINT quickdraw_matches_team_b_fkey FOREIGN KEY (event_id, player_b_team_id) REFERENCES public.teams(event_id, id);


--
-- Name: teams teams_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.teams
    ADD CONSTRAINT teams_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id);


--
-- PostgreSQL database dump complete
--


