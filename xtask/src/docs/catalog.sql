SELECT json_build_object(
  'tables', COALESCE((SELECT json_agg(t ORDER BY t.schema, t.name) FROM (
    SELECT n.nspname AS schema, c.relname AS name,
      obj_description(c.oid) AS comment,
      COALESCE((SELECT json_agg(json_build_object(
        'name', a.attname, 'type', format_type(a.atttypid, a.atttypmod),
        'nullable', NOT a.attnotnull,
        'default', pg_get_expr(d.adbin, d.adrelid),
        'comment', col_description(c.oid, a.attnum)) ORDER BY a.attnum)
        FROM pg_attribute a LEFT JOIN pg_attrdef d
          ON d.adrelid = a.attrelid AND d.adnum = a.attnum
        WHERE a.attrelid = c.oid AND a.attnum > 0 AND NOT a.attisdropped), '[]') AS columns,
      COALESCE((SELECT json_agg(json_build_object('name', conname,
        'definition', pg_get_constraintdef(oid)) ORDER BY conname)
        FROM pg_constraint WHERE conrelid = c.oid), '[]') AS constraints,
      COALESCE((SELECT json_agg(pg_get_indexdef(indexrelid) ORDER BY indexrelid::regclass::text)
        FROM pg_index WHERE indrelid = c.oid), '[]') AS indexes
    FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE c.relkind IN ('r', 'p') AND n.nspname NOT IN ('pg_catalog', 'information_schema')
      AND n.nspname NOT LIKE 'pg_toast%'
  ) t), '[]'),
  'relationships', COALESCE((SELECT json_agg(json_build_object(
    'from', sn.nspname || '.' || s.relname,
    'to', tn.nspname || '.' || t.relname,
    'definition', pg_get_constraintdef(k.oid)) ORDER BY sn.nspname, s.relname, k.conname)
    FROM pg_constraint k
    JOIN pg_class s ON s.oid = k.conrelid JOIN pg_namespace sn ON sn.oid = s.relnamespace
    JOIN pg_class t ON t.oid = k.confrelid JOIN pg_namespace tn ON tn.oid = t.relnamespace
    WHERE k.contype = 'f' AND sn.nspname NOT IN ('pg_catalog', 'information_schema')), '[]'),
  'enums', COALESCE((SELECT json_agg(e ORDER BY e.name) FROM (
    SELECT n.nspname || '.' || t.typname AS name,
      json_agg(e.enumlabel ORDER BY e.enumsortorder) AS values
    FROM pg_type t JOIN pg_namespace n ON n.oid = t.typnamespace
    JOIN pg_enum e ON e.enumtypid = t.oid GROUP BY n.nspname, t.typname
  ) e), '[]')
);
