/// Generate a ~10KB JSON string for benchmarking.
pub fn generate_json() -> String {
    let mut s = String::from("[\n");
    for i in 0..100 {
        if i > 0 {
            s.push_str(",\n");
        }
        s.push_str(&format!(
            r#"  {{
    "id": {i},
    "name": "item_{i}",
    "active": {active},
    "score": {score},
    "tags": ["alpha", "beta", "gamma"],
    "meta": {{ "created": "2025-01-{day:02}", "version": null }}
  }}"#,
            i = i,
            active = if i % 2 == 0 { "true" } else { "false" },
            score = i as f64 * 1.5,
            day = (i % 28) + 1,
        ));
    }
    s.push_str("\n]");
    s
}
