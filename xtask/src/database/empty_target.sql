-- Refuse application objects even when their tables contain no rows.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_namespace
        WHERE nspname <> 'public' AND nspname <> 'information_schema'
          AND nspname !~ '^pg_'
    ) OR EXISTS (
        -- Namespace dependencies include tables, routines, types, collations,
        -- operators and text-search objects without an object-type inventory.
        SELECT 1 FROM pg_depend d JOIN pg_namespace n ON n.oid = d.refobjid
        WHERE d.refclassid = 'pg_namespace'::regclass
          AND n.nspname <> 'information_schema' AND n.nspname !~ '^pg_'
    ) OR EXISTS (
        SELECT 1 FROM pg_extension WHERE extname <> 'plpgsql'
    ) OR EXISTS (SELECT 1 FROM pg_largeobject_metadata)
      OR EXISTS (SELECT 1 FROM pg_event_trigger)
      OR EXISTS (SELECT 1 FROM pg_publication)
      OR EXISTS (SELECT 1 FROM pg_subscription)
    THEN
        RAISE EXCEPTION 'Restore requires a fresh, empty database';
    END IF;
END
$$;
