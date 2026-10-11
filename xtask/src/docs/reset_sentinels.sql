-- Unrelated objects owned by the same role as the application. Broad schema or
-- ownership-based resets must fail the before/after comparison.
CREATE TYPE public.provider_status AS ENUM ('ready');
CREATE TABLE public.provider_metadata (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    status public.provider_status NOT NULL
);
INSERT INTO public.provider_metadata (status) VALUES ('ready');
CREATE VIEW public.provider_metadata_view AS SELECT * FROM public.provider_metadata;
CREATE FUNCTION public.provider_ready() RETURNS boolean
    LANGUAGE sql AS 'SELECT true';
GRANT SELECT ON public.provider_metadata TO PUBLIC;

-- Same names in another schema must survive the public-only reset.
CREATE SCHEMA provider;
CREATE TABLE provider.players (id integer PRIMARY KEY);
INSERT INTO provider.players VALUES (42);
CREATE TABLE provider._sqlx_migrations (version bigint PRIMARY KEY);
INSERT INTO provider._sqlx_migrations VALUES (99);
CREATE TYPE provider.player_role AS ENUM ('provider');
