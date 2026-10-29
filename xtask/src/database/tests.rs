use super::*;
use std::{
    thread,
    time::{Duration, Instant},
};

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "alleycat-recovery-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn failed_dump_never_publishes_output_or_credentials() {
    let directory = TempDir::new();
    let mut command = Command::new("sh");
    command.args([
        "-c",
        "printf 'incomplete SQL'; printf 'secret-password' >&2; exit 1",
    ]);
    let error = backup(
        command,
        BackupArgs {
            output: directory.0.clone(),
        },
    )
    .unwrap_err();
    assert!(!error.to_string().contains("secret-password"));
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 0);
}

#[test]
fn cloud_credentials_are_only_in_the_child_environment() {
    let url =
        Url::parse("postgres://user:secret-password@db.ondigitalocean.com/db?sslmode=require")
            .unwrap();
    let command = cloud_client(&url, "pg_dump");
    assert!(
        command
            .get_args()
            .all(|arg| !arg.to_string_lossy().contains("secret-password"))
    );
    assert!(
        command
            .get_envs()
            .any(|(key, value)| key == "DATABASE_URL" && value == Some(url.as_str().as_ref()))
    );
}

struct Database(String);
impl Drop for Database {
    fn drop(&mut self) {
        let output = Command::new("docker").args(["rm", "-f", &self.0]).output();
        if !output.is_ok_and(|o| o.status.success()) {
            eprintln!(
                "Remove recovery test container with docker rm -f {}",
                self.0
            );
        }
    }
}

fn checked(command: &mut Command) -> String {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn assert_same_dump(expected: &str, actual: &str) {
    for (index, (left, right)) in expected.lines().zip(actual.lines()).enumerate() {
        assert_eq!(left, right, "dump differs at line {}", index + 1);
    }
    assert_eq!(expected.lines().count(), actual.lines().count());
}

impl Database {
    fn client(&self, database: &str, program: &str) -> Command {
        self.client_as(database, program, "postgres")
    }

    fn client_as(&self, database: &str, program: &str, role: &str) -> Command {
        // Exercise the real cloud client, adding only an isolated container
        // network. No REMOTE_DB_URL, .env or application credentials are read.
        let url = Url::parse(&format!(
            "postgres://{role}@127.0.0.1/{database}?sslmode=disable"
        ))
        .unwrap();
        let template = cloud_client(&url, program);
        let mut command = Command::new("docker");
        command.args(["run", "--network", &format!("container:{}", self.0)]);
        command.args(template.get_args().skip(1));
        for (key, value) in template.get_envs() {
            command.env(key, value.unwrap());
        }
        command
    }

    fn sql(&self, database: &str, sql: &str) -> String {
        checked(self.client(database, "psql").args([
            "-X",
            "-A",
            "-t",
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            sql,
        ]))
    }

    fn dump(&self, database: &str) -> String {
        let output = checked(self.client(database, "pg_dump").args([
            "--no-owner",
            "--no-privileges",
            "--no-tablespaces",
        ]));
        output
            .lines()
            .filter(|line| !line.starts_with("\\restrict ") && !line.starts_with("\\unrestrict "))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[test]
#[ignore = "requires Docker and SQLx CLI 0.8.6; creates only disposable local databases"]
fn whole_database_recovery() {
    let directory = TempDir::new();
    let name = format!(
        "alleycat-recovery-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    );
    checked(Command::new("docker").args([
        "run",
        "--detach",
        "--rm",
        "--name",
        &name,
        "--env",
        "POSTGRES_HOST_AUTH_METHOD=trust",
        "--env",
        "POSTGRES_DB=source",
        "--publish",
        "127.0.0.1::5432",
        "--tmpfs",
        "/var/lib/postgresql",
        "postgres:18",
    ]));
    let db = Database(name);
    let start = Instant::now();
    loop {
        if Command::new("docker")
            .args([
                "exec",
                &db.0,
                "pg_isready",
                "-h",
                "127.0.0.1",
                "-U",
                "postgres",
                "-d",
                "source",
            ])
            .output()
            .unwrap()
            .status
            .success()
        {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "test database did not start"
        );
        thread::sleep(Duration::from_millis(250));
    }
    let address = checked(Command::new("docker").args(["port", &db.0, "5432/tcp"]));
    let source_url = format!(
        "postgres://postgres@{}/source?sslmode=disable",
        address.trim()
    );
    let migrate = |url: &str| {
        checked(
            Command::new("sqlx")
                .args(["migrate", "run", "--no-dotenv"])
                .env("DATABASE_URL", url)
                .current_dir(project_root().unwrap()),
        )
    };
    migrate(&source_url);
    db.sql("source", include_str!("../../seed_db.sql"));
    db.sql("source", include_str!("recovery_fixture.sql"));
    for name in ["reference", "recovered", "broken", "occupied"] {
        db.sql(
            "source",
            &format!("CREATE DATABASE {name} TEMPLATE template0"),
        );
    }
    db.sql(
        "source",
        "CREATE ROLE recovery_owner LOGIN; ALTER DATABASE recovered OWNER TO recovery_owner",
    );
    // pg_dump/restore can flatten equivalent CHECK expressions. Use a direct
    // standard-tools round trip as the expected schema, without using xtask's
    // backup or restore for the reference. All rows and objects are compared.
    let reference = directory.0.join("reference.sql");
    fs::write(&reference, db.dump("source")).unwrap();
    checked(
        db.client("reference", "psql")
            .args([
                "-X",
                "-v",
                "ON_ERROR_STOP=1",
                "--single-transaction",
                "--file=-",
            ])
            .stdin(File::open(&reference).unwrap()),
    );
    let before = db.dump("reference");
    db.sql(
        "source",
        "CREATE ROLE backup_reader LOGIN; GRANT pg_read_all_data TO backup_reader",
    );
    let path = backup(
        db.client_as("source", "pg_dump", "backup_reader"),
        BackupArgs {
            output: directory.0.clone(),
        },
    )
    .unwrap();
    let sql = fs::read_to_string(&path).unwrap();
    assert!(sql.contains("-- Operator checkout:"));
    assert!(sql.contains("-- Dumped from database version 18"));
    assert!(sql.contains("-- Dumped by pg_dump version 18"));
    assert!(sql.contains("COPY public._sqlx_migrations"));
    assert!(!sql.contains("postgres://"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    restore(db.client_as("recovered", "psql", "recovery_owner"), &path).unwrap();
    // Compare every object, row and sequence without a central table inventory.
    assert_same_dump(&before, &db.dump("recovered"));
    assert_eq!(db.sql("recovered", "SELECT count(*) FROM devices d JOIN device_inspections i ON i.device_id=d.id JOIN pdn_inspection_checks c ON c.inspection_id=i.id").trim(), "3");
    // SQLx must accept the restored checksums without reapplying migrations.
    migrate(&source_url.replace("/source?", "/recovered?"));
    assert_same_dump(&before, &db.dump("recovered"));

    assert!(restore(db.client("recovered", "psql"), &path).is_err());
    assert_same_dump(&before, &db.dump("recovered"));
    db.sql(
        "occupied",
        "CREATE TABLE unrelated (id int); INSERT INTO unrelated VALUES (42)",
    );
    let occupied = db.dump("occupied");
    assert!(restore(db.client("occupied", "psql"), &path).is_err());
    assert_same_dump(&occupied, &db.dump("occupied"));
    db.sql("occupied", "DELETE FROM unrelated");
    let occupied = db.dump("occupied");
    assert!(restore(db.client("occupied", "psql"), &path).is_err());
    assert_same_dump(&occupied, &db.dump("occupied"));
    db.sql(
        "occupied",
        "DROP TABLE unrelated; CREATE COLLATION custom_collation FROM \"C\"",
    );
    let occupied = db.dump("occupied");
    assert!(restore(db.client("occupied", "psql"), &path).is_err());
    assert_same_dump(&occupied, &db.dump("occupied"));

    // An error after the entire dump must undo schema, rows and the ledger.
    let broken = directory.0.join("broken.sql");
    fs::write(&broken, format!("{sql}\nSELECT 1 / 0;\n")).unwrap();
    let empty = db.dump("broken");
    assert!(restore(db.client("broken", "psql"), &broken).is_err());
    assert_same_dump(&empty, &db.dump("broken"));
    restore(db.client("broken", "psql"), &path).unwrap();
    assert_same_dump(&before, &db.dump("broken"));

    let failed_dir = directory.0.join("failed");
    assert!(
        backup(
            db.client("does_not_exist", "pg_dump"),
            BackupArgs {
                output: failed_dir.clone()
            }
        )
        .is_err()
    );
    assert_eq!(fs::read_dir(failed_dir).unwrap().count(), 0);
}
