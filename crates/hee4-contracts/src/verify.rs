//! The VERIFY line grammar: the only home of the rule that turns a brief's VERIFY text into
//! lines the driver can run. K6's playbook maps [`VerifyLine`] exhaustively; it does not parse.
//!
//! Admission reads the text, not the effect (see `FLOW.md`, "What `check_verify` does not
//! catch"): a line that runs a real command and proves nothing is the verdict's business. A
//! line whose exit status cannot be non-zero is admission's business: [`VerifyLine::is_no_op`]
//! reads the shell text lexically (never the filesystem) and says so for a listed no-op after
//! normalisation, a compound of listed no-ops, and a forced exit (`|| true`, `; exit 0`, a
//! trailing `&`, a pipe into `tail` or `head`). A line it cannot read (a subshell, a group, a
//! compound keyword, `$(..)`, `${..}`, a here-document) counts as a line that may fail.

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

/// Programs that succeed and print nothing, after lexical path normalisation.
const TRUE_WORDS: [&str; 3] = ["true", "/bin/true", "/usr/bin/true"];

/// Programs that print their arguments and succeed: a no-op whatever the args.
const ECHO_WORDS: [&str; 3] = ["echo", "/bin/echo", "/usr/bin/echo"];

/// `env <program> ..` runs `<program>` (an option or an assignment after `env` is not read).
const ENV_WORDS: [&str; 3] = ["env", "/bin/env", "/usr/bin/env"];

/// `<shell> -c <script>` runs `<script>`; the script is read with the same rules.
const SHELL_WORDS: [&str; 6] = [
    "sh",
    "/bin/sh",
    "/usr/bin/sh",
    "bash",
    "/bin/bash",
    "/usr/bin/bash",
];

/// Filters whose exit status is their own. The dispatcher runs a `sh:` line as `/bin/sh -c
/// <command>` with no `pipefail` (`hee4-app/src/dispatcher.rs`, `playbook`), so a pipeline that
/// ends in one of these cannot report the failure of the command before the pipe.
const MASKING_FILTERS: [&str; 6] = [
    "tail",
    "/bin/tail",
    "/usr/bin/tail",
    "head",
    "/bin/head",
    "/usr/bin/head",
];

/// POSIX special built-ins and the other built-ins that can end the shell or change how it
/// treats an error. An error in a special built-in ends a non-interactive shell, so a later
/// command's status is not the line's: a line using one (other than a plain `:` or `exit 0`)
/// may fail.
const ESCAPES: [&str; 18] = [
    ":", ".", "break", "continue", "eval", "exec", "exit", "export", "readonly", "return", "set",
    "shift", "times", "trap", "unset", "source", "kill", "shopt",
];

/// Words that open a compound command. A line that uses one is not read; it may fail.
const RESERVED: [&str; 22] = [
    "if", "then", "else", "elif", "fi", "do", "done", "case", "esac", "while", "until", "for",
    "in", "{", "}", "!", "[[", "]]", "function", "select", "time", "coproc",
];

/// How deep `sh -c 'sh -c ..'` is read before the line counts as one that may fail.
const MAX_DEPTH: u8 = 4;

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

    /// Whether the line runs and proves nothing because its exit status cannot be non-zero:
    /// `true`, `:`, `exit 0`, `exit`, an echo, an exec of `true`/`echo`; the same after
    /// normalisation (quotes, `//`, `.` and `..` in a path, a trailing `;` or `# comment`,
    /// `env <no-op>`, `sh -c <no-op>`); a compound whose every command is one of those; or a
    /// forced exit (`<cmd> || true`, `<cmd>; exit 0`, `<cmd> &`, `<cmd> | tail -1`). An
    /// [`VerifyLine::Unsupported`] line is never a no-op (it never runs).
    #[must_use]
    pub fn is_no_op(&self) -> bool {
        let class = match self {
            Self::Shell { command } => shell_class(command, 0),
            Self::Exec { program, args } => exec_class(program, args),
            Self::Unsupported { .. } => return false,
        };
        class != Class::CanFail
    }
}

/// What a runnable line can do, read from its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    /// Every command in it is a listed no-op.
    NoOp,
    /// It runs a real command, but its exit status cannot be non-zero.
    CannotFail,
    /// Its exit status can be non-zero, or the text was not read.
    CanFail,
}

/// One shell command, judged alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A listed no-op: succeeds and proves nothing.
    NoOp,
    /// `exit 0`: ends the shell with status 0.
    ExitZero,
    /// A bare `exit`: ends the shell with the previous status.
    ExitBare,
    /// `sh -c <script>` whose script cannot fail.
    CannotFail,
    /// A `tail` or `head`: its status is its own (it masks only at the end of a pipeline).
    Filter,
    /// A built-in that can end the shell or change its error handling.
    Escape,
    /// A compound keyword: the line is not read.
    Opaque,
    /// Any other command: it can fail.
    Other,
}

/// The statuses a command, pipeline or list can end with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Status {
    ok: bool,
    fail: bool,
}

const OK: Status = Status {
    ok: true,
    fail: false,
};

const ANY: Status = Status {
    ok: true,
    fail: true,
};

/// A control operator between commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Pipe,
    And,
    Or,
    Seq,
    Background,
}

/// One shell word after quote removal; `dynamic` when its value depends on an expansion.
#[derive(Debug, Default)]
struct Word {
    text: String,
    dynamic: bool,
}

#[derive(Debug)]
enum Token {
    Word(Word),
    /// A redirection operator; `dup` when it ends in `&` (`>&`, `<&`, `2>&`), so its target is
    /// a file descriptor, not a file.
    Redirect {
        dup: bool,
    },
    Op(Op),
}

/// The redirections on one command, as far as they decide whether it can fail. Ordered: a
/// command's redirections are the worst of its parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Redir {
    /// None.
    None,
    /// Only redirections that cannot fail for any command: to or from `/dev/null`, or a dup
    /// onto fd 1 or 2.
    Null,
    /// A dup onto fd 0 or a close (`-`): harmless for a command that writes nothing (`true`,
    /// `:`), but a write to the moved or closed fd fails, so `echo` can fail. The fd the
    /// redirection applies to is not read: `echo x 2>&-` counts as one that can fail, which
    /// admits a line and never refuses one.
    Silent,
    /// Any other: a file that may not open, an fd that may be closed, or no target.
    Other,
}

impl Redir {
    /// Add one redirection whose target is `target` (`None`: the line ended or an operator came
    /// first).
    fn add(self, dup: bool, target: Option<&Word>) -> Self {
        let this = match target {
            Some(w) if !w.dynamic && dup => match w.text.as_str() {
                "1" | "2" => Self::Null,
                "0" | "-" => Self::Silent,
                _ => Self::Other,
            },
            Some(w) if !w.dynamic && w.text == "/dev/null" => Self::Null,
            _ => Self::Other,
        };
        self.max(this)
    }
}

/// One `;`- or `&`-terminated list: its and-or chain of pipelines (each with the operator that
/// joins it to the one before), each pipeline its commands' kinds in order.
#[derive(Debug)]
struct List {
    chain: Vec<(Op, Vec<Kind>)>,
    background: bool,
}

/// Collapse `//`, `.` and `..` in an absolute path without touching the filesystem; any other
/// word is returned as is.
fn lexical_path(word: &str) -> String {
    if !word.starts_with('/') {
        return word.to_owned();
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in word.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    format!("/{}", parts.join("/"))
}

fn flush(word: &mut Option<Word>, tokens: &mut Vec<Token>) {
    if let Some(w) = word.take() {
        tokens.push(Token::Word(w));
    }
}

/// Read one redirection operator starting at `c` (`<` or `>`) into `tokens`. `None` for a
/// here-document, which this reader does not follow.
fn lex_redirect(
    c: char,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    word: &mut Option<Word>,
    tokens: &mut Vec<Token>,
) -> Option<()> {
    // A word of digits right before the operator is its fd (`2>`), not an argument.
    if word.as_ref().is_some_and(|w| {
        !w.dynamic && !w.text.is_empty() && w.text.bytes().all(|b| b.is_ascii_digit())
    }) {
        *word = None;
    }
    flush(word, tokens);
    if c == '<' && chars.peek() == Some(&'<') {
        return None;
    }
    let mut last = c;
    while let Some(n) = chars.next_if(|n| matches!(n, '<' | '>' | '&' | '|')) {
        last = n;
    }
    tokens.push(Token::Redirect { dup: last == '&' });
    Some(())
}

/// Split a shell script into words, redirections and control operators. `None` when the text
/// uses something this reader does not follow (a subshell, `$(..)`, `${..}`, a backtick, a
/// here-document, `;;`, an unterminated quote).
fn lex(script: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut word: Option<Word> = None;
    let mut chars = script.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => flush(&mut word, &mut tokens),
            '\n' => {
                flush(&mut word, &mut tokens);
                tokens.push(Token::Op(Op::Seq));
            }
            '#' if word.is_none() => break,
            '\'' => {
                let w = word.get_or_insert_with(Word::default);
                loop {
                    match chars.next()? {
                        '\'' => break,
                        ch => w.text.push(ch),
                    }
                }
            }
            '"' => {
                let w = word.get_or_insert_with(Word::default);
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => {
                            let n = chars.next()?;
                            if !matches!(n, '$' | '`' | '"' | '\\' | '\n') {
                                w.text.push('\\');
                            }
                            w.text.push(n);
                        }
                        '`' => return None,
                        '$' => {
                            if matches!(chars.peek(), Some('(' | '{')) {
                                return None;
                            }
                            w.dynamic = true;
                            w.text.push('$');
                        }
                        ch => w.text.push(ch),
                    }
                }
            }
            '\\' => {
                let w = word.get_or_insert_with(Word::default);
                w.text.push(chars.next().unwrap_or('\\'));
            }
            '$' => {
                if matches!(chars.peek(), Some('(' | '{')) {
                    return None;
                }
                let w = word.get_or_insert_with(Word::default);
                w.dynamic = true;
                w.text.push('$');
            }
            '`' | '(' | ')' => return None,
            '|' => {
                flush(&mut word, &mut tokens);
                let op = if chars.next_if_eq(&'|').is_some() {
                    Op::Or
                } else {
                    // `|&` pipes stderr too (bash): still a pipe.
                    chars.next_if_eq(&'&');
                    Op::Pipe
                };
                tokens.push(Token::Op(op));
            }
            '&' if chars.peek() == Some(&'>') => {
                // `&>` and `&>>` (bash): stdout and stderr to a file.
                flush(&mut word, &mut tokens);
                while chars.next_if_eq(&'>').is_some() {}
                tokens.push(Token::Redirect { dup: false });
            }
            '&' => {
                flush(&mut word, &mut tokens);
                let op = if chars.next_if_eq(&'&').is_some() {
                    Op::And
                } else {
                    Op::Background
                };
                tokens.push(Token::Op(op));
            }
            ';' => {
                flush(&mut word, &mut tokens);
                if chars.peek() == Some(&';') {
                    return None;
                }
                tokens.push(Token::Op(Op::Seq));
            }
            '<' | '>' => lex_redirect(c, &mut chars, &mut word, &mut tokens)?,
            ch => word.get_or_insert_with(Word::default).text.push(ch),
        }
    }
    flush(&mut word, &mut tokens);
    Some(tokens)
}

/// Group tokens into lists of and-or chains of pipelines, judging each command as it closes.
/// `None` on a syntax error: an operator with no command before it, or a trailing `|`, `&&` or
/// `||`. A trailing `;` is allowed.
fn lists(tokens: Vec<Token>, depth: u8) -> Option<Vec<List>> {
    let mut out = Vec::new();
    let mut chain: Vec<(Op, Vec<Kind>)> = Vec::new();
    let mut pipeline: Vec<Kind> = Vec::new();
    let mut words: Vec<Word> = Vec::new();
    let mut redirects = Redir::None;
    // A redirection waiting for its target word.
    let mut pending: Option<bool> = None;
    let mut join = Op::Seq;
    for token in tokens {
        match token {
            Token::Word(w) => match pending.take() {
                Some(dup) => redirects = redirects.add(dup, Some(&w)),
                None => words.push(w),
            },
            Token::Redirect { dup } => {
                if let Some(dup) = pending.replace(dup) {
                    redirects = redirects.add(dup, None);
                }
            }
            Token::Op(op) => {
                if let Some(dup) = pending.take() {
                    redirects = redirects.add(dup, None);
                }
                if words.is_empty() && redirects == Redir::None {
                    return None;
                }
                pipeline.push(kind(&words, redirects, false, depth));
                words.clear();
                redirects = Redir::None;
                if op == Op::Pipe {
                    continue;
                }
                chain.push((join, std::mem::take(&mut pipeline)));
                if matches!(op, Op::And | Op::Or) {
                    join = op;
                    continue;
                }
                out.push(List {
                    chain: std::mem::take(&mut chain),
                    background: op == Op::Background,
                });
                join = Op::Seq;
            }
        }
    }
    if let Some(dup) = pending.take() {
        redirects = redirects.add(dup, None);
    }
    if words.is_empty() && redirects == Redir::None {
        return (pipeline.is_empty() && chain.is_empty()).then_some(out);
    }
    pipeline.push(kind(&words, redirects, false, depth));
    chain.push((join, pipeline));
    out.push(List {
        chain,
        background: false,
    });
    Some(out)
}

/// Judge one command. `argv` is true for an exec (no shell: no built-ins, no keywords).
///
/// A no-op whose only redirections cannot fail (`true 2>/dev/null`, `true >&-`, `echo x >&2`)
/// is still a no-op; any other redirection makes the command one that can fail. `echo` writes,
/// so a dup onto fd 0 or a close (`Redir::Silent`) makes it one that can fail.
fn kind(words: &[Word], redirects: Redir, argv: bool, depth: u8) -> Kind {
    let Some((first, rest)) = words.split_first() else {
        return Kind::Other;
    };
    if first.dynamic {
        return Kind::Other;
    }
    let program = lexical_path(&first.text);
    let p = program.as_str();
    if !argv {
        if RESERVED.contains(&p) {
            return Kind::Opaque;
        }
        if p == "exit" && redirects == Redir::None {
            return match rest {
                [] => Kind::ExitBare,
                [w] if !w.dynamic && w.text == "0" => Kind::ExitZero,
                _ => Kind::Escape,
            };
        }
        if p == ":" && redirects != Redir::Other {
            return Kind::NoOp;
        }
        if ESCAPES.contains(&p) {
            return Kind::Escape;
        }
    }
    if (TRUE_WORDS.contains(&p) && redirects != Redir::Other)
        || (ECHO_WORDS.contains(&p) && redirects <= Redir::Null)
    {
        return Kind::NoOp;
    }
    if redirects != Redir::None {
        return Kind::Other;
    }
    if ENV_WORDS.contains(&p) {
        return match rest.first() {
            Some(w) if !w.dynamic && !w.text.starts_with('-') && !w.text.contains('=') => {
                kind(rest, Redir::None, true, depth)
            }
            _ => Kind::Other,
        };
    }
    if SHELL_WORDS.contains(&p)
        && let [flag, script, ..] = rest
        && flag.text == "-c"
        && !flag.dynamic
        && !script.dynamic
        && depth < MAX_DEPTH
    {
        return match shell_class(&script.text, depth + 1) {
            Class::NoOp => Kind::NoOp,
            Class::CannotFail => Kind::CannotFail,
            Class::CanFail => Kind::Other,
        };
    }
    if MASKING_FILTERS.contains(&p) {
        return Kind::Filter;
    }
    Kind::Other
}

/// A pipeline's status is its last command's (no `pipefail`); a filter at the end of a
/// pipeline of two or more masks what came before it.
fn pipeline_status(kinds: &[Kind]) -> Status {
    match kinds {
        [_, .., Kind::Filter] | [.., Kind::NoOp | Kind::ExitZero | Kind::CannotFail] => OK,
        _ => ANY,
    }
}

/// An and-or chain's possible statuses: `a && b` runs `b` only on success, `a || b` only on
/// failure.
fn chain_status(chain: &[(Op, Vec<Kind>)]) -> Status {
    let mut status = OK;
    for (i, (op, pipeline)) in chain.iter().enumerate() {
        let next = pipeline_status(pipeline);
        status = match op {
            _ if i == 0 => next,
            Op::And => Status {
                ok: status.ok && next.ok,
                fail: status.fail || (status.ok && next.fail),
            },
            Op::Or => Status {
                ok: status.ok || (status.fail && next.ok),
                fail: status.fail && next.fail,
            },
            Op::Pipe | Op::Seq | Op::Background => next,
        };
    }
    status
}

/// Read a `sh:` line (or an `sh -c` script) and say whether it can fail.
fn shell_class(script: &str, depth: u8) -> Class {
    let Some(lists) = lex(script).and_then(|t| lists(t, depth)) else {
        return Class::CanFail;
    };
    let kinds = || {
        lists
            .iter()
            .flat_map(|l| l.chain.iter().flat_map(|(_, p)| p.iter().copied()))
    };
    if kinds().any(|k| k == Kind::Opaque) {
        return Class::CanFail;
    }
    if kinds().all(|k| matches!(k, Kind::NoOp | Kind::ExitZero | Kind::ExitBare)) {
        return Class::NoOp;
    }
    let mut status = OK;
    for list in &lists {
        let list_kinds = || list.chain.iter().flat_map(|(_, p)| p.iter().copied());
        if list_kinds().any(|k| k == Kind::Escape) {
            return Class::CanFail;
        }
        if let [(_, pipeline)] = list.chain.as_slice()
            && let [only] = pipeline.as_slice()
        {
            match only {
                // The shell ends here with 0, whatever follows.
                Kind::ExitZero => return Class::CannotFail,
                // The shell ends here with the status so far.
                Kind::ExitBare => break,
                _ => {}
            }
        }
        if list_kinds().any(|k| k == Kind::ExitBare) {
            return Class::CanFail;
        }
        status = if list.background {
            OK
        } else {
            chain_status(&list.chain)
        };
    }
    if status == OK {
        Class::CannotFail
    } else {
        Class::CanFail
    }
}

/// Read an exec line as an argv: no shell, so only `env`, `sh -c` and the no-op programs.
fn exec_class(program: &str, args: &[String]) -> Class {
    let words: Vec<Word> = std::iter::once(program)
        .chain(args.iter().map(String::as_str))
        .map(|text| Word {
            text: text.to_owned(),
            dynamic: false,
        })
        .collect();
    match kind(&words, Redir::None, true, 0) {
        Kind::NoOp => Class::NoOp,
        Kind::CannotFail => Class::CannotFail,
        _ => Class::CanFail,
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
    /// Every runnable line is a no-op or cannot fail ([`VerifyLine::is_no_op`]).
    #[error(
        "is vacuous: all {lines} runnable line(s) cannot fail (true, :, exit 0, echo, || true, ; exit 0)"
    )]
    OnlyNoOps {
        /// Runnable lines, all of them no-ops.
        lines: usize,
    },
}
