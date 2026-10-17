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
-- Name: btree_gist; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS btree_gist WITH SCHEMA public;


--
-- Name: EXTENSION btree_gist; Type: COMMENT; Schema: -; Owner: -
--

COMMENT ON EXTENSION btree_gist IS 'support for indexing common datatypes in GiST';


--
-- Name: player_role; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.player_role AS ENUM (
    'staff',
    'courier',
    'miniboss'
);


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
-- Name: app_state; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.app_state (
    id integer NOT NULL,
    active_event_id uuid,
    CONSTRAINT app_state_id_check CHECK ((id = 1))
);


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
-- Name: player_roles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.player_roles (
    player_id uuid NOT NULL,
    role public.player_role NOT NULL
);


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
    CONSTRAINT players_pdn_code_check CHECK ((pdn_code ~ '^[0-9]{4}$'::text))
);


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
-- Name: app_state app_state_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.app_state
    ADD CONSTRAINT app_state_pkey PRIMARY KEY (id);


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
-- Name: event_scoring_periods event_scoring_periods_no_overlap; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_scoring_periods
    ADD CONSTRAINT event_scoring_periods_no_overlap EXCLUDE USING gist (event_id WITH =, tstzrange(started_at, closes_at, '[)'::text) WITH &&);


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
-- Name: player_roles player_roles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.player_roles
    ADD CONSTRAINT player_roles_pkey PRIMARY KEY (player_id, role);


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
-- Name: app_state app_state_active_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.app_state
    ADD CONSTRAINT app_state_active_event_id_fkey FOREIGN KEY (active_event_id) REFERENCES public.events(id);


--
-- Name: event_scoring_periods event_scoring_periods_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_scoring_periods
    ADD CONSTRAINT event_scoring_periods_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id);


--
-- Name: player_roles player_roles_player_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.player_roles
    ADD CONSTRAINT player_roles_player_id_fkey FOREIGN KEY (player_id) REFERENCES public.players(id) ON DELETE CASCADE;


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


