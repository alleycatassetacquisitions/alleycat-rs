ALTER TABLE affiliation_categories
    ADD COLUMN send_to_devices boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN affiliation_categories.send_to_devices IS
    'Include assignments in this category in device player lookups. Defaults to false; staff responses always include all assignments.';
