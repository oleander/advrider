use std::collections::HashMap;
use std::{env, path};

use colored::*;

fn find_shortest_path<'a>(segments: Vec<String>) -> Vec<String> {
  let budget: HashMap<&str, u32> = HashMap::from([
    ("/f/forums/racing.25", 1),
    ("/f/forums/racing.25/*", 1),
    ("/f/threads/*/*", 1),
    ("/f/threads/*/*/", 1),
    ("/f/threads/*", 1)
  ]);

  log::info!("Segments: {:?}", segments);
  let head = match segments.split_last() {
    None => return vec!["/".to_string()],
    Some((_, head)) => head
  };
  log::info!("Head: {:?}", head);

  let path = head.join("/");
  for (pattern, _) in budget.iter() {
    if glob::Pattern::new(pattern).unwrap().matches(&path) {
      return segments;
    }
  }

  log::info!("Path: {}", path);
  find_shortest_path(head.to_vec())
}
fn main() {
  // read each line from paths.txt
  env_logger::init();
  let content = std::fs::read_to_string("paths.txt").unwrap();
  let paths = content.lines().collect::<Vec<&str>>();

  log::info!("Read paths: {}", paths.len());

  let mut set = std::collections::HashSet::new();
  for path in paths.iter() {
    let seg = path
      .trim()
      .split('/')
      .into_iter()
      .map(|s| s.to_string())
      .collect::<Vec<String>>();
    let paths = find_shortest_path(seg);
    let shortest_path = paths.join("/");
    log::info!("Shortest path: {}", shortest_path);
    set.insert(shortest_path);
  }

  // write to shortest_paths.txt
  let body = set.iter().map(|s| format!("{}\n", s)).collect::<String>();
  std::fs::write("shortest_paths.txt", body).unwrap();

  log::info!("Wrote to shortest_paths.txt");
}
