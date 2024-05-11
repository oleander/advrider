extern crate regex;
use regex::RegexSet;
use spider::compact_str::CompactString;
use std::collections::HashMap;

fn main() {
    let patterns: Vec<CompactString> = vec![
        "^/f/forums/racing\\.25/$".into(),
        "^/f/forums/racing\\.25/page-\\d+/?$".into(),
        "^/f/threads/[^/]+/$".into(),
        "^/f/threads/[^/]+/page-\\d+/?$".into(),
    ];

    let budget = HashMap::from([
        ("/f/forums/racing.25*", 5),
        ("/f/threads/*/page-*", 10),
        ("/f/threads/*", 2),
    ]);

    let set = RegexSet::new(&patterns).unwrap();

    // Example URL to test
    let test_url = "/f/posts/50465159/help/";

    // Check if the URL is blocked
    if set.is_match(test_url) {
        println!("URL is blocked: {}", test_url);
    } else {
        println!("URL is allowed: {}", test_url);
    }

    // Example budget check (pseudo-code, implement actual logic to decrement and check budget)
    let budget_check = |path: &str| -> bool {
        // Simulate checking the budget
        budget.get(path).map_or(false, |&limit| limit > 0)
    };

    // Print if the URL is within budget
    println!("URL is within budget: {}", budget_check(test_url));
}
