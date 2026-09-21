//! The stand-in for a WhatsApp account: `zapzap-core --demo` plays conversations, contacts, calls and
//! stories for the ui. The real connection is `zapzap-engine`.

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--demo") {
        return zapzap_core::demo::run();
    }
    eprintln!("usage: zapzap-core --demo   (the real connection is zapzap-engine; see scripts/run-linux.sh)");
    std::process::exit(2);
}
