-- Replace the unused fixed-role placeholders; no predefined affiliations are seeded.
DROP TABLE player_roles;
DROP TYPE player_role;

CREATE TABLE affiliation_categories (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id uuid NOT NULL REFERENCES events(id),
    name text NOT NULL CHECK (name <> '' AND name = btrim(name) AND char_length(name) <= 256),
    CONSTRAINT affiliation_categories_event_name_key UNIQUE (event_id, name),
    CONSTRAINT affiliation_categories_event_id_key UNIQUE (event_id, id)
);

CREATE TABLE affiliations (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id uuid NOT NULL,
    category_id uuid NOT NULL,
    name text NOT NULL CHECK (name <> '' AND name = btrim(name) AND char_length(name) <= 256),
    CONSTRAINT affiliations_category_fkey FOREIGN KEY (event_id, category_id)
        REFERENCES affiliation_categories(event_id, id),
    CONSTRAINT affiliations_category_name_key UNIQUE (category_id, name),
    CONSTRAINT affiliations_event_category_id_key UNIQUE (event_id, category_id, id)
);

CREATE TABLE player_affiliations (
    event_id uuid NOT NULL,
    player_id uuid NOT NULL,
    category_id uuid NOT NULL,
    affiliation_id uuid NOT NULL,
    PRIMARY KEY (player_id, category_id),
    CONSTRAINT player_affiliations_player_fkey FOREIGN KEY (event_id, player_id)
        REFERENCES players(event_id, id),
    CONSTRAINT player_affiliations_affiliation_fkey FOREIGN KEY (event_id, category_id, affiliation_id)
        REFERENCES affiliations(event_id, category_id, id)
);
CREATE INDEX player_affiliations_affiliation_idx ON player_affiliations(event_id, category_id, affiliation_id);

COMMENT ON TABLE affiliation_categories IS
    'Operator-defined affiliation categories within an event. No seeded categories or implied gameplay behavior.';
COMMENT ON TABLE affiliations IS
    'Operator-defined affiliations within a category. Names have no implicit gameplay or permission behavior.';
COMMENT ON TABLE player_affiliations IS
    'At most one affiliation per player per category. Composite foreign keys keep player, category, and affiliation in the same event. Clearing an assignment deletes its row; changes do not affect scores.';
