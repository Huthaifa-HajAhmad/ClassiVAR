//! Print the review packet for the built-in rules (or a rule file given as the
//! first argument) and the sign-offs in `rules/review-signoff.toml`.
//!
//! `cargo run -p classivar-core --example review_packet > docs/review/REVIEW-PACKET.md`

use classivar_core::RuleSet;
use classivar_core::rules::parse_signoffs;

fn main() {
    let rules = match std::env::args().nth(1) {
        Some(path) => {
            RuleSet::parse(&std::fs::read_to_string(&path).expect("readable rule file")).expect("valid rules")
        }
        None => RuleSet::builtin().expect("valid built-in rules").clone(),
    };
    let text = include_str!("../../../rules/review-signoff.toml");
    let signoffs = parse_signoffs(text).expect("a readable sign-off file");
    if let Err(problem) = rules.check_signoffs(&signoffs) {
        eprintln!("The sign-offs do not fit these rules: {problem}");
        std::process::exit(1);
    }
    print!("{}", rules.review_packet(&signoffs));
}
