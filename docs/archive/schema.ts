import type { Database } from "bun:sqlite";

type Migration = {
	version: number;
	name: string;
	statements: string[];
};

const migrations: Migration[] = [
	{
		version: 1,
		name: "init",
		statements: [
			`
			CREATE TABLE IF NOT EXISTS players (
				id TEXT PRIMARY KEY CHECK(id GLOB '[0-9][0-9][0-9][0-9]'),
				name TEXT,
				hunter BOOLEAN,
				allegiance NUMBER DEFAULT 0,
				faction TEXT DEFAULT '',
				neoId TEXT DEFAULT ''
			)
			`,
			`
			CREATE TABLE IF NOT EXISTS available_ids (
				id TEXT PRIMARY KEY CHECK(id GLOB '[0-9][0-9][0-9][0-9]')
			)
			`,
			`
			WITH RECURSIVE
			numbers(n) AS (
				SELECT 0
				UNION ALL
				SELECT n + 1 FROM numbers WHERE n < 9999
			)
			INSERT OR IGNORE INTO available_ids (id)
			SELECT substr('0000' || n, -4) FROM numbers
			`,
			`
			DELETE FROM available_ids
			WHERE id IN ('0000', '1111', '2222', '3333', '4444', '5555', '6666', '7777', '8888', '9999', '8008', '6969', '1337')
			`,
			`
			CREATE TRIGGER IF NOT EXISTS assign_next_id_on_insert
			AFTER INSERT ON players
			WHEN NEW.id IS NULL
			BEGIN
				UPDATE players
				SET id = (
					SELECT id FROM available_ids ORDER BY RANDOM() LIMIT 1
				)
				WHERE id IS NULL;

				DELETE FROM available_ids
				WHERE id = (
					SELECT id FROM players
					WHERE rowid = NEW.rowid
				);
			END
			`,
			`
			CREATE TABLE IF NOT EXISTS matches (
				match_id TEXT,
				hunter TEXT,
				bounty TEXT,
				hunter_time INTEGER,
				bounty_time INTEGER,
				winner_is_hunter BOOLEAN,
				PRIMARY KEY (match_id),
				FOREIGN KEY (hunter) REFERENCES players(id),
				FOREIGN KEY (bounty) REFERENCES players(id)
			)
			`,
			`
			CREATE INDEX IF NOT EXISTS matches_by_hunter
			ON matches (hunter)
			`,
			`
			CREATE INDEX IF NOT EXISTS matches_by_bounty
			ON matches (bounty)
			`,
			`
			CREATE INDEX IF NOT EXISTS matches_by_winner_hunter
			ON matches (winner_is_hunter, hunter)
			`,
			`
			CREATE INDEX IF NOT EXISTS matches_by_winner_bounty
			ON matches (winner_is_hunter, bounty)
			`,
			`
			CREATE INDEX IF NOT EXISTS players_by_faction
			ON players (faction)
			`,
			`
			CREATE INDEX IF NOT EXISTS players_by_name
			ON players (name)
			`,
		],
	},
	{
		version: 2,
		name: "quests",
		statements: [
			`
			ALTER TABLE players
			ADD COLUMN hacked_boxes_bits INTEGER NOT NULL DEFAULT 0
			`,
			`
				CREATE TABLE IF NOT EXISTS ufr_readers (
					osn TEXT PRIMARY KEY NOT NULL,
					name TEXT NOT NULL
				)
				`,
			`
				CREATE TABLE IF NOT EXISTS quest_givers (
					id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
					name TEXT NOT NULL,
					passcode TEXT NOT NULL,
					UNIQUE (passcode)
				)
				`,
			`
				CREATE TABLE IF NOT EXISTS missions (
					id TEXT PRIMARY KEY,
					quest_giver_id TEXT NOT NULL,
					label TEXT NOT NULL,
					description TEXT NOT NULL DEFAULT '',
					type INTEGER NOT NULL,
					options_json TEXT NOT NULL DEFAULT '{}',
					win_condition TEXT NOT NULL DEFAULT '',
					FOREIGN KEY (quest_giver_id) REFERENCES quest_givers(id) ON DELETE CASCADE
				)
				`,
			`
			CREATE INDEX IF NOT EXISTS missions_by_quest_giver
			ON missions (quest_giver_id)
			`,
			`
					CREATE TABLE IF NOT EXISTS mission_statuses (
						user_id TEXT NOT NULL,
						mission_id TEXT NOT NULL,
						complete INTEGER NOT NULL DEFAULT 0 CHECK (complete IN (0, 1)),
						status_json TEXT NOT NULL DEFAULT '{}',
						started_at INTEGER,
						updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
						completed_at INTEGER,
						PRIMARY KEY (user_id, mission_id),
						FOREIGN KEY (user_id) REFERENCES players(id) ON DELETE CASCADE,
					FOREIGN KEY (mission_id) REFERENCES missions(id) ON DELETE CASCADE
				)
				`,
			`
				CREATE INDEX IF NOT EXISTS mission_statuses_by_mission
				ON mission_statuses (mission_id)
				`,
			`
				CREATE INDEX IF NOT EXISTS matches_by_hunter
				ON matches (hunter)
				`,
			`
				CREATE INDEX IF NOT EXISTS matches_by_bounty
				ON matches (bounty)
				`,
			`
				CREATE INDEX IF NOT EXISTS matches_by_winner_hunter
				ON matches (winner_is_hunter, hunter)
				`,
			`
				CREATE INDEX IF NOT EXISTS matches_by_winner_bounty
				ON matches (winner_is_hunter, bounty)
				`,
			`
				CREATE INDEX IF NOT EXISTS players_by_faction
				ON players (faction)
				`,
			`
				CREATE INDEX IF NOT EXISTS players_by_name
				ON players (name)
				`,
		],
	},
];

export function migrate(db: Database) {
	const { user_version } = db.query("PRAGMA user_version;").get() as {
		user_version: number;
	};
	let currentVersion = user_version ?? 0;

	const ordered = migrations.slice().sort((a, b) => a.version - b.version);
	for (const migration of ordered) {
		if (migration.version <= currentVersion) continue;
		applyMigration(db, migration);
		currentVersion = migration.version;
	}
}

function applyMigration(db: Database, migration: Migration) {
	db.run("BEGIN;");
	try {
		for (const statement of migration.statements) {
			db.run(statement);
		}
		db.run(`PRAGMA user_version = ${migration.version};`);
		db.run("COMMIT;");
	} catch (err) {
		db.run("ROLLBACK;");
		throw err;
	}
}

/**
 * Backwards-compatible alias for older code paths.
 * New code should call `migrate(db)`.
 */
export function createTables(db: Database) {
	migrate(db);
}
