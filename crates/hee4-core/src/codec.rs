//! The durable spelling of an [`Event`]: the contracts' own `Serialize` output, read back by
//! the contracts' own `Deserialize`. A row that is not an `Event` is unreadable and is never
//! repaired (EX-05).

use hee4_contracts::Event;

/// The stored spelling of `event`.
pub(crate) fn encode(event: Event) -> Result<String, serde_json::Error> {
    serde_json::to_string(&event)
}

/// The event a stored spelling names, or `None` for any other text.
pub(crate) fn decode(text: &str) -> Option<Event> {
    serde_json::from_str::<Event>(text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hee4_contracts::{
        AbandonReason, QuarantineReason, RecoveryRule, Resolution, Settlement, Verdict,
    };

    #[test]
    fn events_round_trip_and_a_foreign_spelling_is_unreadable() -> Result<(), serde_json::Error> {
        let sample = [
            Event::Admit,
            Event::Settle(Settlement::Ready),
            Event::Decide(Verdict::Pass),
            Event::Resolve(Resolution::Abandon(AbandonReason::RouteRefused {
                floor_unmet: true,
            })),
            Event::Resolve(Resolution::Quarantine(
                QuarantineReason::EffectUnknownPermanent {
                    rule: RecoveryRule::R07ProcessNotOurs,
                },
            )),
            Event::Recover(RecoveryRule::R14UnexpectedState),
        ];
        for e in sample {
            assert_eq!(decode(&encode(e)?), Some(e));
        }
        assert_eq!(decode(r#""queued""#), None);
        assert_eq!(decode(""), None);
        Ok(())
    }
}
