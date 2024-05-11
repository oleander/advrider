use std::collections::HashMap;

fn main() {
    let mut budget = HashMap::from([
        ("/f/forums/racing.25/*".to_string(), 5),
        ("/f/threads/*/page-*".to_string(), 10),
        ("/f/threads/*".to_string(), 2),
        ("/f/*".to_string(), 1), // Default budget for unmatched paths
    ]);

    let test_urls = [
        "/f/forums/racing.25/page-4",
        "/f/threads/motogp-francais-spoileurs.1733155/page-281",
        "/f/posts/50465159/help/",
    ];

    for url in test_urls.iter() {
        let is_within_budget = check_budget(url, &mut budget);
        println!("URL is within budget ({}) for '{}': {}", is_within_budget, url, budget.get("/f/*").unwrap_or(&0));
    }
}

fn check_budget(url: &str, budget: &mut HashMap<String, i32>) -> bool {
    // Here we would have a more sophisticated matching system
    let pattern = "/f/*"; // Simplified: This would be determined by matching patterns
    budget.get_mut(pattern).map_or(false, |limit| {
        if *limit > 0 {
            *limit -= 1; // Decrement the budget
            true
        } else {
            false
        }
    })
}
