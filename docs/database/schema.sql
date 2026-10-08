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
-- Name: player_role; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.player_role AS ENUM (
    'staff',
    'courier',
    'miniboss'
);


--
-- Name: player_team; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.player_team AS ENUM (
    'unassigned',
    'hunter',
    'bounty'
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
-- Name: events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.events (
    id uuid NOT NULL,
    name text NOT NULL,
    venue_name text,
    next_pdn_code integer DEFAULT 1 NOT NULL,
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT events_next_pdn_code_check CHECK (((next_pdn_code >= 1) AND (next_pdn_code <= 10000)))
);


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
    team public.player_team DEFAULT 'unassigned'::public.player_team NOT NULL,
    email text,
    created_at timestamp with time zone NOT NULL,
    event_id uuid NOT NULL,
    CONSTRAINT players_pdn_code_check CHECK ((pdn_code ~ '^[0-9]{4}$'::text))
);


--
-- Name: reserved_pdn_codes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.reserved_pdn_codes (
    code text NOT NULL,
    reason text,
    CONSTRAINT reserved_pdn_codes_code_check CHECK ((code ~ '^[0-9]{4}$'::text))
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
-- Name: reserved_pdn_codes reserved_pdn_codes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reserved_pdn_codes
    ADD CONSTRAINT reserved_pdn_codes_pkey PRIMARY KEY (code);


--
-- Name: players_event_created_at_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX players_event_created_at_id_idx ON public.players USING btree (event_id, created_at DESC, id DESC);


--
-- Name: app_state app_state_active_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.app_state
    ADD CONSTRAINT app_state_active_event_id_fkey FOREIGN KEY (active_event_id) REFERENCES public.events(id);


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
-- PostgreSQL database dump complete
--


