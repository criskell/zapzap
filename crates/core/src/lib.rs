//! What the ZapZap processes share besides the WhatsApp protocol itself (that comes from the
//! `whatsapp-rust` crates, in `zapzap-engine`): the line protocol to the ui, the voice-call state
//! machine, the audio check, and the demo that stands in for an account.

pub mod audio;
pub mod call;
pub mod contacts;
pub mod demo;
pub mod link;
pub mod reactions;
pub mod trim;
