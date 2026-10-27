//! What the three Server runs share.
//!
//! Upload, Compare with Server and Test connection all reach the Server
//! over ssh, so they read ssh's exit code the same way.

/// ssh's exit code when it couldn't connect or log in.
pub(super) const SSH_FAILED: i32 = 255;

// == Hints ====================================================================

/// Returns the hint for a Server run's command failing with `code`, when
/// it's [`SSH_FAILED`].
pub(super) fn ssh_hint(code: i32) -> Option<String> {
    return (code == SSH_FAILED).then(|| {
        "couldn't reach or log in to the Server; check its host, port, user and SSH key".to_owned()
    });
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;

    /// Only ssh's own failure gets the hint.
    #[test]
    fn only_ssh_failed_has_a_hint() {
        assert_eq!(
            ssh_hint(SSH_FAILED).as_deref(),
            Some("couldn't reach or log in to the Server; check its host, port, user and SSH key")
        );
        assert_eq!(ssh_hint(23), None);
    }
}
