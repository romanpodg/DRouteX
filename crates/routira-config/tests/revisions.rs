use routira_common::{ConfigurationRevisionId, CorrelationId, UserId};
use routira_config::{DraftRevision, InvalidRevisionMetadata};
use std::{
    str::FromStr,
    time::{Duration, SystemTime},
};

fn id<T: FromStr>(number: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{number:012x}")
        .parse()
        .unwrap()
}

#[test]
fn revision_context_rejects_self_parent_and_invalid_clock_values() {
    let revision: ConfigurationRevisionId = id(1);
    let author: UserId = id(2);
    let correlation: CorrelationId = id(3);
    assert_eq!(
        DraftRevision::from_unix_millis(revision, Some(revision), author, 0, correlation),
        Err(InvalidRevisionMetadata::SelfParent)
    );
    assert_eq!(
        DraftRevision::from_unix_millis(revision, None, author, -1, correlation),
        Err(InvalidRevisionMetadata::Timestamp)
    );
    assert_eq!(
        DraftRevision::new(
            revision,
            None,
            author,
            SystemTime::UNIX_EPOCH - Duration::from_secs(1),
            correlation
        ),
        Err(InvalidRevisionMetadata::Timestamp)
    );
    let metadata = DraftRevision::new(
        revision,
        None,
        author,
        SystemTime::UNIX_EPOCH + Duration::from_millis(1234),
        correlation,
    )
    .unwrap();
    assert_eq!(metadata.unix_millis(), 1234);
    let earlier_clock =
        DraftRevision::from_unix_millis(id(4), Some(revision), author, 1000, correlation).unwrap();
    assert_eq!(earlier_clock.parent(), Some(revision));
}
