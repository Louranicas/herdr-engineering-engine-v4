//! The VERIFY line grammar: the only home of the rule that turns a brief's VERIFY text into
//! lines the driver can run. K6's playbook maps [`VerifyLine`] exhaustively; it does not parse.
//!
//! Admission reads the text, not the effect (see `FLOW.md`, "What `check_verify` does not
//! catch"): a line that runs a real command and proves nothing is the verdict's business.

/// One VERIFY line after normalisation (`trim`, a leading `- ` stripped, backticks stripped).
/// Exhaustive on purpose: a fourth variant must fail to compile in every mapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyLine {
    /// `sh: <command>`: runs as `/bin/sh -c <command>` in the namespace.
    Shell {
        /// The text after `sh:`, trimmed.
        command: String,
    },
    /// An absolute path: runs as a bare argv, split on whitespace.
    Exec {
        /// The first word.
        program: String,
        /// The remaining words.
        args: Vec<String>,
    },
    /// Anything else, named by its first word (`model:` is `model`): the driver records a skip.
    Unsupported {
        /// The first whitespace word, or `""`.
        kind: String,
    },
}

/// Shell commands that run and prove nothing, matched exactly after trimming.
const SHELL_NO_OPS: [&str; 7] = [
    "",
    "true",
    ":",
    "exit",
    "exit 0",
    "/bin/true",
    "/usr/bin/true",
];

/// First words that make a shell command an echo: a no-op unless an operator follows.
const ECHO_WORDS: [&str; 3] = ["echo", "/bin/echo", "/usr/bin/echo"];

/// Any of these in an echo command makes it a pipeline or a redirection, not a no-op.
const SHELL_OPERATORS: [char; 8] = ['|', ';', '&', '<', '>', '(', ')', '`'];

/// Programs whose bare exec proves nothing whatever the args.
const EXEC_NO_OPS: [&str; 4] = ["/bin/true", "/usr/bin/true", "/bin/echo", "/usr/bin/echo"];

impl VerifyLine {
    /// Every non-empty line of a VERIFY field, in order. `model:` lines are
    /// `Unsupported { kind: "model" }`: the driver makes no model call.
    #[must_use]
    pub fn parse_all(verify: &str) -> Vec<Self> {
        verify
            .lines()
            .map(|l| l.trim().trim_start_matches("- ").trim_matches('`'))
            .filter(|l| !l.is_empty())
            .map(Self::parse_one)
            .collect()
    }

    fn parse_one(line: &str) -> Self {
        if line.strip_prefix("model:").is_some() {
            Self::Unsupported {
                kind: "model".to_owned(),
            }
        } else if let Some(cmd) = line.strip_prefix("sh:") {
            Self::Shell {
                command: cmd.trim().to_owned(),
            }
        } else if line.starts_with('/') {
            let mut words = line.split_whitespace().map(str::to_owned);
            Self::Exec {
                program: words.next().unwrap_or_default(),
                args: words.collect(),
            }
        } else {
            Self::Unsupported {
                kind: line.split_whitespace().next().unwrap_or("").to_owned(),
            }
        }
    }

    /// Whether the driver would run anything for this line.
    #[must_use]
    pub const fn runs(&self) -> bool {
        matches!(self, Self::Shell { .. } | Self::Exec { .. })
    }

    /// Whether the line runs and proves nothing: `true`, `:`, `exit 0`, a bare echo, or an exec
    /// of `true`/`echo`. An [`VerifyLine::Unsupported`] line is never a no-op (it never runs).
    #[must_use]
    pub fn is_no_op(&self) -> bool {
        match self {
            Self::Shell { command } => {
                let cmd = command.trim();
                SHELL_NO_OPS.contains(&cmd)
                    || (cmd
                        .split_whitespace()
                        .next()
                        .is_some_and(|word| ECHO_WORDS.contains(&word))
                        && !cmd.contains(&SHELL_OPERATORS[..]))
            }
            Self::Exec { program, .. } => EXEC_NO_OPS.contains(&program.as_str()),
            Self::Unsupported { .. } => false,
        }
    }
}

/// Why a VERIFY field looks at nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum VerifyFault {
    /// No line survives normalisation.
    #[error("is empty")]
    Empty,
    /// Lines exist; none is a `sh:` line or an absolute path.
    #[error("runs nothing: {lines} line(s), none is an absolute path or `sh:`")]
    NothingRuns {
        /// Lines after normalisation.
        lines: usize,
    },
    /// Every runnable line is a no-op.
    #[error("is vacuous: all {lines} runnable line(s) are no-ops (true, :, exit 0, echo)")]
    OnlyNoOps {
        /// Runnable lines, all of them no-ops.
        lines: usize,
    },
}
