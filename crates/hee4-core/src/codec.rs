//! The durable spelling of an [`Event`]: the contracts' own `Serialize` output, decoded by
//! lookup over the finite event space (`Event` has no `Deserialize`, and this crate does not
//! redefine it). A row that is not one of the 32 spellings decodes to `None` (EX-05).

use std::sync::OnceLock;

use hee4_contracts::{Event, Reason, RecoveryRule, Resolution, Settlement, Verdict};

/// Every value of `Event` (32, as `hee4-contracts/FLOW.md` counts them).
pub(crate) fn every_event() -> Vec<Event> {
    let mut all = vec![Event::Admit, Event::Dispatch, Event::Observe];
    all.extend(
        [
            Settlement::Ready,
            Settlement::NotReady,
            Settlement::Unsettled,
        ]
        .map(Event::Settle),
    );
    all.extend([Verdict::Pass, Verdict::Fail].map(Event::Decide));
    all.extend(Reason::ALL.map(|r| Event::Decide(Verdict::Refused(r))));
    all.extend([Event::Accept, Event::Cancel]);
    all.extend([Resolution::Quarantine, Resolution::Abandon].map(Event::Resolve));
    all.push(Event::Stop);
    all.extend(RecoveryRule::ALL.map(Event::Recover));
    all
}

fn table() -> &'static [(String, Event)] {
    static TABLE: OnceLock<Vec<(String, Event)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        every_event()
            .into_iter()
            .filter_map(|e| serde_json::to_string(&e).ok().map(|s| (s, e)))
            .collect()
    })
}

/// The stored spelling of `event`.
pub(crate) fn encode(event: Event) -> Result<String, serde_json::Error> {
    serde_json::to_string(&event)
}

/// The event a stored spelling names, or `None` for any other text.
pub(crate) fn decode(text: &str) -> Option<Event> {
    table().iter().find(|(s, _)| s == text).map(|(_, e)| *e)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Which `Event` variant, by position. No wildcard arm: a new variant in `hee4-contracts`
    /// stops this crate's tests compiling until [`every_event`] covers it.
    const fn variant(event: Event) -> usize {
        match event {
            Event::Admit => 0,
            Event::Dispatch => 1,
            Event::Observe => 2,
            Event::Settle(_) => 3,
            Event::Decide(_) => 4,
            Event::Accept => 5,
            Event::Cancel => 6,
            Event::Resolve(_) => 7,
            Event::Stop => 8,
            Event::Recover(_) => 9,
        }
    }

    /// The number of `Event` variants [`variant`] numbers.
    const VARIANTS: usize = 10;

    #[test]
    fn every_event_round_trips_and_covers_every_variant() -> Result<(), serde_json::Error> {
        let all = every_event();
        assert_eq!(all.len(), 32);
        let mut seen = [false; VARIANTS];
        for e in &all {
            seen[variant(*e)] = true;
            assert_eq!(decode(&encode(*e)?), Some(*e));
        }
        assert_eq!(seen, [true; VARIANTS]);
        assert_eq!(table().len(), 32, "32 distinct spellings");
        assert_eq!(decode(r#""queued""#), None);
        Ok(())
    }
}
