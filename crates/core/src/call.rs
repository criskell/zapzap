//! The life of a voice call, apart from how it travels: what the ui may do when, and what the ui is
//! told. The demo feeds it scripted events; the real connection will feed it the call stanzas.
//! Every line it returns is one line of the protocol documented in the ui's `handle_line`.

use crate::link::clock;

pub type Lines = Vec<Vec<String>>;

/// What can happen to a call, from the ui or from the other side.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The ui placed a call.
    Dial(String),
    /// The ui answered the incoming call.
    Answer,
    /// The ui declined, cancelled or ended the call.
    Hangup,
    /// Someone calls us.
    Incoming(String),
    /// The other side picked up the call we placed.
    PeerAnswered,
    /// The other side ended, declined or cancelled the call; `reason` is shown when we were ringing.
    PeerEnded(String),
    /// Nobody picked up in time.
    RingTimeout,
    /// Another device of ours answered or declined the incoming call: it stops ringing, no missed call.
    AnsweredElsewhere,
}

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Idle,
    Incoming(String),
    Ringing(String),
    Active { name: String, we_called: bool },
}

/// What the call needs from the world besides events.
pub struct Audio {
    pub microphone: bool,
    pub speaker: bool,
}

pub struct Calls {
    phase: Phase,
}

/// The lines for the ui, and whether a call that came in while another was going must be rejected
/// as busy on the wire.
#[derive(Default)]
pub struct Outcome {
    pub lines: Lines,
    pub reject_busy: bool,
}

fn line(fields: &[&str]) -> Vec<String> {
    fields.iter().map(|field| field.to_string()).collect()
}

/// A call that just ended, for the top of the recents. `kind`: 0 received, 1 missed, 2 outgoing.
fn logged(kind: &str, name: &str) -> Vec<String> {
    line(&["NEWCALL", kind, "1", &clock(), name])
}

const NO_MICROPHONE: [&str; 2] = [
    "Microfone não encontrado",
    "Não é possível fazer ligações porque parece que seu computador não tem um microfone. Experimente conectar um microfone ou, caso já tenha conectado, reinicie o ZapZap.",
];
const NO_SPEAKER: [&str; 2] = [
    "Alto-falante não encontrado",
    "Não é possível fazer ligações porque parece que seu computador não tem uma saída de áudio. Experimente conectar um alto-falante ou fones de ouvido.",
];

impl Calls {
    pub fn new() -> Self {
        Calls { phase: Phase::Idle }
    }

    pub fn is_idle(&self) -> bool {
        self.phase == Phase::Idle
    }

    /// The name of the other side, while a call is going.
    pub fn peer(&self) -> Option<&str> {
        match &self.phase {
            Phase::Idle => None,
            Phase::Incoming(name) | Phase::Ringing(name) | Phase::Active { name, .. } => Some(name),
        }
    }

    fn missing_audio(audio: &Audio) -> Option<Lines> {
        let [title, body] = if !audio.microphone {
            NO_MICROPHONE
        } else if !audio.speaker {
            NO_SPEAKER
        } else {
            return None;
        };
        Some(vec![line(&["CALLSTATE", "failed", title, body])])
    }

    pub fn handle(&mut self, event: Event, audio: &Audio) -> Outcome {
        let mut out = Outcome::default();
        match (std::mem::replace(&mut self.phase, Phase::Idle), event) {
            (Phase::Idle, Event::Dial(name)) => match Self::missing_audio(audio) {
                Some(lines) => out.lines = lines,
                None => self.phase = Phase::Ringing(name),
            },
            (Phase::Idle, Event::Incoming(name)) => {
                out.lines.push(line(&["INCOMING", &name]));
                self.phase = Phase::Incoming(name);
            }
            (Phase::Incoming(name), Event::Answer) => match Self::missing_audio(audio) {
                Some(lines) => out.lines = lines,
                None => self.phase = Phase::Active { name, we_called: false },
            },
            (Phase::Incoming(_), Event::Hangup) => {}
            (Phase::Incoming(name), Event::PeerEnded(_)) => {
                out.lines.push(line(&["CALLSTATE", "ended"]));
                out.lines.push(logged("1", &name));
            }
            (Phase::Incoming(_), Event::AnsweredElsewhere) => out.lines.push(line(&["CALLSTATE", "ended"])),
            (Phase::Ringing(name), Event::PeerAnswered) => {
                out.lines.push(line(&["CALLSTATE", "active"]));
                self.phase = Phase::Active { name, we_called: true };
            }
            (Phase::Ringing(name), Event::RingTimeout) => {
                out.lines.push(line(&["CALLSTATE", "ended", "Sem resposta"]));
                out.lines.push(logged("2", &name));
            }
            (Phase::Ringing(name), Event::PeerEnded(reason)) => {
                out.lines.push(line(&["CALLSTATE", "ended", &reason]));
                out.lines.push(logged("2", &name));
            }
            (Phase::Ringing(name), Event::Hangup) => out.lines.push(logged("2", &name)),
            (Phase::Active { name, we_called }, Event::Hangup) => out.lines.push(logged(if we_called { "2" } else { "0" }, &name)),
            (Phase::Active { name, we_called }, Event::PeerEnded(_)) => {
                out.lines.push(line(&["CALLSTATE", "ended"]));
                out.lines.push(logged(if we_called { "2" } else { "0" }, &name));
            }
            // A call while another is going: turned away, the current one goes on.
            (phase, Event::Incoming(_)) => {
                out.reject_busy = true;
                self.phase = phase;
            }
            // Anything else does not apply in this phase (a late timeout, a second hang-up...).
            (phase, _) => self.phase = phase,
        }
        out
    }
}

impl Default for Calls {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FINE: Audio = Audio { microphone: true, speaker: true };

    fn first(out: &Outcome) -> Vec<&str> {
        out.lines.first().map(|line| line.iter().map(String::as_str).collect()).unwrap_or_default()
    }

    #[test]
    fn a_call_we_place_rings_then_connects_and_is_logged_as_outgoing() {
        let mut calls = Calls::new();
        assert!(calls.handle(Event::Dial("Mãe".into()), &FINE).lines.is_empty());
        assert_eq!(calls.peer(), Some("Mãe"));
        assert_eq!(first(&calls.handle(Event::PeerAnswered, &FINE)), ["CALLSTATE", "active"]);
        let ended = calls.handle(Event::Hangup, &FINE);
        assert_eq!(&ended.lines[0][..3], ["NEWCALL", "2", "1"]);
        assert!(calls.is_idle());
    }

    #[test]
    fn nobody_answering_ends_the_call_with_a_reason_and_logs_it() {
        let mut calls = Calls::new();
        calls.handle(Event::Dial("Bruno".into()), &FINE);
        let out = calls.handle(Event::RingTimeout, &FINE);
        assert_eq!(out.lines[0], ["CALLSTATE", "ended", "Sem resposta"]);
        assert_eq!(out.lines[1][1], "2");
        assert!(calls.is_idle());
    }

    #[test]
    fn a_busy_peer_is_reported_with_its_reason() {
        let mut calls = Calls::new();
        calls.handle(Event::Dial("Bruno".into()), &FINE);
        assert_eq!(calls.handle(Event::PeerEnded("Ocupado".into()), &FINE).lines[0], ["CALLSTATE", "ended", "Ocupado"]);
    }

    #[test]
    fn an_incoming_call_that_stops_ringing_is_a_missed_call() {
        let mut calls = Calls::new();
        assert_eq!(first(&calls.handle(Event::Incoming("Mãe".into()), &FINE)), ["INCOMING", "Mãe"]);
        let out = calls.handle(Event::PeerEnded(String::new()), &FINE);
        assert_eq!(out.lines[0], ["CALLSTATE", "ended"]);
        assert_eq!(&out.lines[1][..3], ["NEWCALL", "1", "1"]);
        assert!(calls.is_idle());
    }

    #[test]
    fn an_incoming_call_taken_by_another_device_is_not_missed() {
        let mut calls = Calls::new();
        calls.handle(Event::Incoming("Mãe".into()), &FINE);
        let out = calls.handle(Event::AnsweredElsewhere, &FINE);
        assert_eq!(out.lines, [["CALLSTATE", "ended"]]);
        assert!(calls.is_idle());
    }

    #[test]
    fn an_answered_incoming_call_is_logged_as_received() {
        let mut calls = Calls::new();
        calls.handle(Event::Incoming("Ana".into()), &FINE);
        assert!(calls.handle(Event::Answer, &FINE).lines.is_empty());
        assert_eq!(calls.handle(Event::Hangup, &FINE).lines[0][1], "0");
    }

    #[test]
    fn a_second_call_is_turned_away_without_disturbing_the_first() {
        let mut calls = Calls::new();
        calls.handle(Event::Dial("Mãe".into()), &FINE);
        let out = calls.handle(Event::Incoming("Ana".into()), &FINE);
        assert!(out.reject_busy && out.lines.is_empty());
        assert_eq!(calls.peer(), Some("Mãe"));
    }

    #[test]
    fn without_a_microphone_nothing_rings_and_the_ui_is_told_why() {
        let mut calls = Calls::new();
        let out = calls.handle(Event::Dial("Mãe".into()), &Audio { microphone: false, speaker: true });
        assert_eq!(&out.lines[0][..3], ["CALLSTATE", "failed", "Microfone não encontrado"]);
        assert!(calls.is_idle());
        calls.handle(Event::Incoming("Ana".into()), &FINE);
        let out = calls.handle(Event::Answer, &Audio { microphone: true, speaker: false });
        assert_eq!(out.lines[0][2], "Alto-falante não encontrado");
        assert!(calls.is_idle());
    }

    #[test]
    fn late_or_repeated_events_change_nothing() {
        let mut calls = Calls::new();
        assert!(calls.handle(Event::RingTimeout, &FINE).lines.is_empty());
        assert!(calls.handle(Event::Hangup, &FINE).lines.is_empty());
        calls.handle(Event::Dial("Mãe".into()), &FINE);
        assert!(calls.handle(Event::Answer, &FINE).lines.is_empty());
        assert_eq!(calls.peer(), Some("Mãe"));
    }

    #[test]
    fn no_field_can_break_the_line_format() {
        let mut calls = Calls::new();
        let out = calls.handle(Event::Dial("x".into()), &Audio { microphone: false, speaker: true });
        assert!(out.lines.iter().flatten().all(|field| !field.contains('\t') && !field.contains('\n')));
    }
}
