use std::ffi::OsString;
use std::path::PathBuf;

pub const HELP: &str = "SkySheet CLI Alpha — inspect and validate score JSON

Usage:
  skysheet --help
  skysheet --version
  skysheet inspect <FILE> [--json]
  skysheet validate <FILE> [--json]

Options:
  -h, --help     Show help (also accepted with inspect/validate)
  -V, --version  Show program version
  --json         Emit one JSON result, including errors
  --             Treat remaining arguments as positional (for filenames starting with -)

Both commands read original JSON and run Score validation; input files are never modified.
Exit codes: 0 success; 1 invalid score; 2 usage; 3 I/O; 4 internal error.
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    Inspect,
    Validate,
}

impl Command {
    pub fn name(self) -> &'static str {
        match self {
            Self::Help => "help",
            Self::Version => "version",
            Self::Inspect => "inspect",
            Self::Validate => "validate",
        }
    }
}

#[derive(Debug)]
pub struct Args {
    pub command: Command,
    pub file: Option<PathBuf>,
}

// Detect output mode even when argument parsing fails. After --, --json is a filename.
pub fn json_requested(args: &[OsString]) -> bool {
    args.iter()
        .take_while(|arg| *arg != "--")
        .any(|arg| arg == "--json")
}

pub fn parse(args: &[OsString]) -> Result<Args, String> {
    let mut positional = Vec::new();
    let mut literal = false;
    let mut help = false;
    let mut version = false;
    let mut json = false;
    for arg in args {
        if !literal {
            match arg.to_str() {
                Some("--") => {
                    literal = true;
                    continue;
                }
                Some("--json") if !json => {
                    json = true;
                    continue;
                }
                Some("--help" | "-h") if !help => {
                    help = true;
                    continue;
                }
                Some("--version" | "-V") if !version => {
                    version = true;
                    continue;
                }
                _ if arg.to_string_lossy().starts_with('-') => {
                    return Err(format!("unknown or repeated option {arg:?}; use --help"));
                }
                _ => {}
            }
        }
        positional.push(arg);
    }
    if help && version {
        return Err("--help and --version cannot be combined".into());
    }
    if positional.is_empty() {
        let command = if help {
            Command::Help
        } else if version {
            Command::Version
        } else {
            return Err("missing command; use --help".into());
        };
        return Ok(Args {
            command,
            file: None,
        });
    }
    let command = match positional[0].to_str() {
        Some("inspect") => Command::Inspect,
        Some("validate") => Command::Validate,
        _ => {
            return Err(format!(
                "unsupported command {:?}; use --help",
                positional[0]
            ));
        }
    };
    if version {
        return Err("--version must be used without a subcommand".into());
    }
    if help && positional.len() == 1 {
        return Ok(Args {
            command: Command::Help,
            file: None,
        });
    }
    if positional.len() != 2 || help {
        return Err(format!(
            "usage: skysheet {} <FILE> [--json]",
            command.name()
        ));
    }
    Ok(Args {
        command,
        file: Some(PathBuf::from(positional[1])),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_option_filename_does_not_enable_json() {
        let args = ["inspect", "--", "--json"].map(OsString::from);
        assert!(!json_requested(&args));
        assert_eq!(parse(&args).unwrap().file.unwrap(), PathBuf::from("--json"));
    }

    #[test]
    fn invalid_arguments_still_select_machine_diagnostics() {
        let args = ["unknown", "--json"].map(OsString::from);
        assert!(json_requested(&args));
        assert!(parse(&args).is_err());
    }
}
