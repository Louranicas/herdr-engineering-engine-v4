//! The principal a request acts as. Defined in contracts (A24, STORE-G10) so that `actions` names it
//! without importing `store`; the store re-exports it and still owns what is persisted for it.

/// Stable principal supplied by the authenticated owner, never decoded from a body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Principal {
    uid: u32,
    role: String,
}

/// Why a configured principal was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrincipalError {
    /// The role is empty, longer than 64 bytes, or not `[A-Za-z0-9_-]`.
    InvalidRole,
}

impl Principal {
    /// # Errors
    /// Refuses an empty, oversized or non-identifier configured role.
    pub fn new(uid: u32, role: &str) -> Result<Self, PrincipalError> {
        if role.is_empty()
            || role.len() > 64
            || !role
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(PrincipalError::InvalidRole);
        }
        Ok(Self {
            uid,
            role: role.to_owned(),
        })
    }
    /// Whether this is the principal with `uid` and the configured `role`: how a record naming
    /// its principal (a grant) is matched without exposing either field.
    #[must_use]
    pub fn is(&self, uid: u32, role: &str) -> bool {
        self.uid == uid && self.role == role
    }
    #[must_use]
    pub(crate) fn uid(&self) -> u32 {
        self.uid
    }
    #[must_use]
    pub(crate) fn role(&self) -> &str {
        &self.role
    }
    /// The outbox recipient key this principal's events are addressed to.
    #[must_use]
    pub(crate) fn recipient(&self) -> String {
        format!("{}:{}", self.uid, self.role)
    }
}

/// The one configured local role the operator is served under (IPC01): the socket admits its peer
/// as this role, and the store's "operator only" rule admits exactly this role (R21 N12). One
/// spelling, below both doors.
pub const OPERATOR_ROLE: &str = "operator";
