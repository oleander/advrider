use std::collections::HashMap;
use colored::*;

fn main() {
  let budget = HashMap::from([
    ("/f/forums/racing.25".to_string(), 5),
    ("/f/forums/racing.25/*".to_string(), 5),
    ("/f/threads/*/*".to_string(), 10),
    ("/f/threads/*/*/".to_string(), 10),
    ("/f/threads/*".to_string(), 2)
  ]);

  // /f/{forums,threads}/*{/,}*
  let patterns = vec!["^/f/members".to_string(), "^/f/posts".to_string()];

  let urls = [
    ("/f/forums/racing.25/page-4", true, false),
    ("/f/threads/motogp-francais-spoileurs.1733155/page-281", true, false),
    ("/f/posts/50465159/help/", false, true),
    ("/f/members/123", false, true),
    ("/f/forums/racing.25/", true, false),
    ("/f/forums/racing.25", true, false)
  ];

  for (url, include, exclude) in urls.iter() {
    println!("URL: {}", url.cyan());
    for pattern in patterns.iter() {
      let hit = regex::Regex::new(pattern).unwrap().is_match(url);
      let actual = *exclude == hit;
      println!("\t[{}] Regex: {}", b(actual), pattern.yellow(),);
    }

    for glob in budget.keys() {
      let hit = glob::Pattern::new(glob).unwrap().matches(url);
      let actual = *include == hit;
      println!("\t[{}] Glob: {}", b(actual), glob.yellow());
    }
  }
}

fn b(v: bool) -> String {
  if v {
    "Yes".green().to_string()
  } else {
    "No".red().to_string()
  }
}
