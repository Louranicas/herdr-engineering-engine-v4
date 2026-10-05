//! The I1 brief: eleven fields, parsed once at admission.

use crate::refusal::Refusal;
use crate::verify::{VerifyFault, VerifyLine};

/// The eleven brief fields (STACK-MAP §2 I1: pstack's nine plus RECON and RESTATEMENT).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)] // each variant is its label
pub enum BriefField {
    Goal,
    Scope,
    Context,
    Acceptance,
    Verify,
    Timebox,
    Forbidden,
    Report,
    Standing,
    Recon,
    Restatement,
}

impl BriefField {
    /// Every field, in brief order.
    pub const ALL: [Self; 11] = [
        Self::Goal,
        Self::Scope,
        Self::Context,
        Self::Acceptance,
        Self::Verify,
        Self::Timebox,
        Self::Forbidden,
        Self::Report,
        Self::Standing,
        Self::Recon,
        Self::Restatement,
    ];

    /// The label as written in a brief, without the colon.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Goal => "GOAL",
            Self::Scope => "SCOPE",
            Self::Context => "CONTEXT",
            Self::Acceptance => "ACCEPTANCE",
            Self::Verify => "VERIFY",
            Self::Timebox => "TIMEBOX",
            Self::Forbidden => "FORBIDDEN",
            Self::Report => "REPORT",
            Self::Standing => "STANDING",
            Self::Recon => "RECON",
            Self::Restatement => "RESTATEMENT",
        }
    }

    const fn index(self) -> usize {
        self as usize
    }

    /// The field a line opens, and the text after `LABEL:`.
    fn opens(line: &str) -> Option<(Self, &str)> {
        Self::ALL.into_iter().find_map(|field| {
            line.strip_prefix(field.label())
                .and_then(|rest| rest.strip_prefix(':'))
                .map(|rest| (field, rest))
        })
    }
}

/// A brief with all eleven fields present. A value of this type cannot lack one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brief {
    fields: [String; 11],
}

impl Brief {
    /// Parse the text form: each field opens on a line starting `LABEL:`; following lines that
    /// open no field continue it. Text before the first field is a preamble and is not kept.
    ///
    /// # Errors
    /// [`Refusal::DuplicateBriefField`] for a field opened twice; then
    /// [`Refusal::MissingBriefField`] naming the first absent field in brief order.
    pub fn parse(text: &str) -> Result<Self, Refusal> {
        let mut found: [Option<String>; 11] = Default::default();
        let mut current: Option<BriefField> = None;
        for line in text.lines() {
            if let Some((field, rest)) = BriefField::opens(line) {
                let slot = &mut found[field.index()];
                if slot.is_some() {
                    return Err(Refusal::DuplicateBriefField { field });
                }
                *slot = Some(rest.trim().to_owned());
                current = Some(field);
            } else if let Some(field) = current
                && let Some(value) = found[field.index()].as_mut()
            {
                value.push('\n');
                value.push_str(line);
            }
        }
        let mut fields: [String; 11] = Default::default();
        for (field, (out, value)) in BriefField::ALL
            .into_iter()
            .zip(fields.iter_mut().zip(found))
        {
            let value = value.ok_or(Refusal::MissingBriefField { field })?;
            value.trim().clone_into(out);
        }
        Ok(Self { fields })
    }

    /// The text of `field` (possibly empty: presence is guaranteed, content is the reader's).
    #[must_use]
    pub fn get(&self, field: BriefField) -> &str {
        &self.fields[field.index()]
    }

    /// The admission check of narrative principle 13: the worker restated the goal.
    ///
    /// # Errors
    /// [`Refusal::EmptyRestatement`] when RESTATEMENT is empty or whitespace.
    pub fn check_restatement(&self) -> Result<&str, Refusal> {
        let text = self.get(BriefField::Restatement);
        if text.trim().is_empty() {
            Err(Refusal::EmptyRestatement)
        } else {
            Ok(text)
        }
    }

    /// The admission check of V4-94: VERIFY looks at something. Reads the text only, never the
    /// host, the sandbox or a file; what it deliberately does not catch is listed in `FLOW.md`.
    ///
    /// # Errors
    /// [`Refusal::VacuousVerify`] with [`VerifyFault::Empty`] when no line survives
    /// normalisation; [`VerifyFault::NothingRuns`] when no line is `sh:` or an absolute path;
    /// [`VerifyFault::OnlyNoOps`] when every runnable line is a no-op. One real runnable line
    /// anywhere admits.
    pub fn check_verify(&self) -> Result<&str, Refusal> {
        let text = self.get(BriefField::Verify);
        let lines = VerifyLine::parse_all(text);
        if lines.is_empty() {
            return Err(Refusal::VacuousVerify {
                cause: VerifyFault::Empty,
            });
        }
        let runnable: Vec<&VerifyLine> = lines.iter().filter(|l| l.runs()).collect();
        if runnable.is_empty() {
            return Err(Refusal::VacuousVerify {
                cause: VerifyFault::NothingRuns { lines: lines.len() },
            });
        }
        if runnable.iter().all(|l| l.is_no_op()) {
            return Err(Refusal::VacuousVerify {
                cause: VerifyFault::OnlyNoOps {
                    lines: runnable.len(),
                },
            });
        }
        Ok(text)
    }
}
