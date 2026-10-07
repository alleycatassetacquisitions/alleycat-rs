use crate::project::project_root;
use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command as Process, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Subcommand)]
pub enum Command {
    /// Export schema.sql and README.md using a disposable PostgreSQL 18 container.
    Db {
        /// Output directory, relative to the repository root unless absolute.
        #[arg(long, default_value = "docs/database")]
        output: PathBuf,
    },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Db { output } => generate(output),
    }
}

// Drop cleans up after both success and ordinary error returns. No ports or volumes
// are published, and this container never uses the application's credentials.
struct Database(String);
impl Drop for Database {
    fn drop(&mut self) {
        if !Process::new("docker")
            .args(["rm", "-f", &self.0])
            .output()
            .is_ok_and(|o| o.status.success())
        {
            eprintln!(
                "Could not remove documentation container {}; remove it with docker rm -f {}",
                self.0, self.0
            );
        }
    }
}

fn docker(args: &[&str], input: Option<&[u8]>) -> Result<String> {
    let mut child = Process::new("docker")
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("could not start Docker; install Docker and start its daemon")?;
    let write_result = if let Some(input) = input {
        child
            .stdin
            .take()
            .context("missing Docker stdin")?
            .write_all(input)
    } else {
        Ok(())
    };
    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!("Docker command failed with {}", output.status);
    }
    write_result?;
    Ok(String::from_utf8(output.stdout)?)
}

fn generate(output: PathBuf) -> Result<()> {
    let root = project_root()?;
    let output = root.join(output);
    let name = format!(
        "alleycat-docs-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    println!("Starting temporary PostgreSQL 18 for database documentation...");
    docker(
        &[
            "run",
            "--detach",
            "--rm",
            "--name",
            &name,
            "--network",
            "none",
            "--env",
            "POSTGRES_HOST_AUTH_METHOD=trust",
            "--env",
            "POSTGRES_DB=docs",
            "--tmpfs",
            "/var/lib/postgresql",
            "postgres:18",
        ],
        None,
    )?;
    let db = Database(name);
    let start = Instant::now();
    loop {
        if Process::new("docker")
            // The image's bootstrap server only listens on a Unix socket.
            // Wait for TCP so migrations cannot race its final restart.
            .args([
                "exec",
                &db.0,
                "pg_isready",
                "-h",
                "127.0.0.1",
                "-U",
                "postgres",
                "-d",
                "docs",
            ])
            .output()?
            .status
            .success()
        {
            break;
        }
        if start.elapsed() > Duration::from_secs(60) {
            bail!("temporary PostgreSQL did not become ready in 60 seconds");
        }
        thread::sleep(Duration::from_millis(250));
    }
    let mut migrations = fs::read_dir(root.join("migrations"))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    migrations.retain(|p| {
        p.extension().is_some_and(|e| e == "sql") && !p.to_string_lossy().ends_with(".down.sql")
    });
    migrations.sort();
    if migrations.is_empty() {
        bail!("no SQL migrations found");
    }
    for migration in migrations {
        println!(
            "Applying {}",
            migration.file_name().unwrap().to_string_lossy()
        );
        docker(
            &[
                "exec",
                "-i",
                &db.0,
                "psql",
                "-X",
                "-q",
                "-v",
                "ON_ERROR_STOP=1",
                "-U",
                "postgres",
                "-d",
                "docs",
            ],
            Some(&fs::read(&migration)?),
        )
        .with_context(|| format!("failed to apply {}", migration.display()))?;
    }
    let schema = docker(
        &[
            "exec",
            &db.0,
            "pg_dump",
            "-U",
            "postgres",
            "-d",
            "docs",
            "--schema-only",
            "--no-owner",
            "--no-privileges",
        ],
        None,
    )?;
    let catalog = docker(
        &[
            "exec",
            "-i",
            &db.0,
            "psql",
            "-X",
            "-A",
            "-t",
            "-v",
            "ON_ERROR_STOP=1",
            "-U",
            "postgres",
            "-d",
            "docs",
        ],
        Some(include_bytes!("catalog.sql")),
    )?;
    let markdown = render(&serde_json::from_str(&catalog)?);
    fs::create_dir_all(&output)?;
    fs::write(output.join("schema.sql"), normalize_dump(&schema))?;
    fs::write(output.join("README.md"), markdown)?;
    println!(
        "Generated {} and {}",
        output.join("schema.sql").display(),
        output.join("README.md").display()
    );
    Ok(())
}

fn normalize_dump(input: &str) -> String {
    input
        .lines()
        .filter(|line| {
            !line.starts_with("\\restrict ")
                && !line.starts_with("\\unrestrict ")
                && !line.starts_with("-- Dumped from database version")
                && !line.starts_with("-- Dumped by pg_dump version")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}
fn cell(value: &Value) -> String {
    value
        .as_str()
        .unwrap_or("")
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('|', "&#124;")
        .replace('`', "&#96;")
        .replace('\r', "")
        .replace('\n', "<br>")
}
fn diagram_label(s: &str) -> String {
    s.replace('&', "#38;")
        .replace('"', "#34;")
        .replace('<', "#60;")
        .replace('>', "#62;")
        .replace(['\n', '\r'], " ")
}
fn render(catalog: &Value) -> String {
    let tables = catalog["tables"].as_array().expect("catalog tables");
    let names: Vec<String> = tables
        .iter()
        .map(|t| {
            format!(
                "{}.{}",
                t["schema"].as_str().unwrap(),
                t["name"].as_str().unwrap()
            )
        })
        .collect();
    let mut out = String::from(
        "# Database schema\n\nGenerated from the repository’s migrations. After changing migrations, run `cargo xtask docs db` with Docker running, then commit the updated documentation. Do not edit this file manually.\n\n[View the complete SQL schema](schema.sql).\n\n## Relationships\n\nArrows point from the referencing table to the referenced table. Labels show foreign key definitions; cardinality is not inferred.\n\n```mermaid\nflowchart LR\n",
    );
    for (i, name) in names.iter().enumerate() {
        out.push_str(&format!("    t{i}[\"{}\"]\n", diagram_label(name)));
    }
    for r in catalog["relationships"].as_array().unwrap() {
        if let (Some(from), Some(to)) = (
            names
                .iter()
                .position(|n| Some(n.as_str()) == r["from"].as_str()),
            names
                .iter()
                .position(|n| Some(n.as_str()) == r["to"].as_str()),
        ) {
            out.push_str(&format!(
                "    t{from} -->|\"{}\"| t{to}\n",
                diagram_label(r["definition"].as_str().unwrap())
            ));
        }
    }
    out.push_str("```\n\n## Enum types\n\n| Type | Values |\n| --- | --- |\n");
    for e in catalog["enums"].as_array().unwrap() {
        out.push_str(&format!(
            "| {} | {} |\n",
            cell(&e["name"]),
            e["values"]
                .as_array()
                .unwrap()
                .iter()
                .map(cell)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    for (t, name) in tables.iter().zip(names) {
        out.push_str(&format!("\n## {}\n\n{}\n\n| Column | Type | Nullable | Default | Description |\n| --- | --- | --- | --- | --- |\n", cell(&Value::String(name)), cell(&t["comment"])));
        for c in t["columns"].as_array().unwrap() {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                cell(&c["name"]),
                cell(&c["type"]),
                if c["nullable"] == true { "yes" } else { "no" },
                cell(&c["default"]),
                cell(&c["comment"])
            ));
        }
        out.push_str("\n### Constraints\n\n| Name | Definition |\n| --- | --- |\n");
        for c in t["constraints"].as_array().unwrap() {
            out.push_str(&format!(
                "| {} | {} |\n",
                cell(&c["name"]),
                cell(&c["definition"])
            ));
        }
        out.push_str("\n### Indexes\n\n| Definition |\n| --- |\n");
        for i in t["indexes"].as_array().unwrap() {
            out.push_str(&format!("| {} |\n", cell(i)));
        }
    }
    out
}
