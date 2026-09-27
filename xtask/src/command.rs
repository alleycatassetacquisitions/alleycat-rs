use std::process::Command;

pub(crate) fn command_output(program: &str, arguments: &[&str]) -> Result<String, String> {
    match Command::new(program).args(arguments).output() {
        Ok(output) if output.status.success() => {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            if stderr.is_empty() {
                Err(format!("`{program}` exited with {}", output.status))
            } else {
                Err(stderr)
            }
        }
        Err(error) => Err(format!("could not run `{program}`: {error}")),
    }
}
