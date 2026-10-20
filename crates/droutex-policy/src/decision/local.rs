use super::ActionRefusal;
use crate::{CanonicalDomain, PreparedPrivateRecord, PrivateRecordValue};
use droutex_common::{PrivateDnsRecordId, ProfileId};
use std::fmt;

/// Borrowed local data classification, not a complete DNS response or gateway
/// target grant. P2-03 must validate the full RRset; P3-04 owns application/order.
/// It cannot outlive the prepared record:
/// ```compile_fail
/// use droutex_policy::{PreparedPrivateRecord, CanonicalDomain, LocalAnswerIntent, local_answer_intent};
/// fn escape<'a>(record: PreparedPrivateRecord, query: &CanonicalDomain) -> LocalAnswerIntent<'a> {
///     local_answer_intent(&record, record.profile, query).unwrap()
/// }
/// ```
pub struct LocalAnswerIntent<'a> {
    record: &'a PreparedPrivateRecord,
}
impl LocalAnswerIntent<'_> {
    pub fn profile(&self) -> ProfileId {
        self.record.profile
    }
    pub fn record_id(&self) -> PrivateDnsRecordId {
        self.record.id
    }
    pub fn ttl(&self) -> u32 {
        self.record.ttl
    }
    pub fn value(&self) -> &PrivateRecordValue {
        &self.record.value
    }
}
impl fmt::Debug for LocalAnswerIntent<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalAnswerIntent")
            .field("profile", &self.record.profile)
            .field("record", &self.record.id)
            .field("ttl", &self.record.ttl)
            .finish_non_exhaustive()
    }
}

/// Metadata guards prevent accidental cross-Profile/name use; public IDs alone
/// cannot authenticate a requester or establish Profile visibility.
pub fn local_answer_intent<'a>(
    record: &'a PreparedPrivateRecord,
    expected_profile: ProfileId,
    query: &CanonicalDomain,
) -> Result<LocalAnswerIntent<'a>, ActionRefusal> {
    if record.profile != expected_profile {
        return Err(ActionRefusal::PrivateProfileMismatch);
    }
    if &record.owner != query {
        return Err(ActionRefusal::PrivateNameMismatch);
    }
    Ok(LocalAnswerIntent { record })
}
