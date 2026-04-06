pub fn generate_json() -> String {
    let mut out = String::from("[\n");

    for i in 0..100 {
        if i > 0 {
            out.push_str(",\n");
        }

        out.push_str(&format!(
            r#"  {{
    "id": {i},
    "name": "item_{i}",
    "active": {active},
    "score": {score},
    "tags": ["alpha", "beta", "gamma"],
    "meta": {{ "created": "2025-01-{day:02}", "version": null }}
  }}"#,
            active = if i % 2 == 0 { "true" } else { "false" },
            score = i as f64 * 1.5,
            day = (i % 28) + 1,
        ));
    }

    out.push_str("\n]");
    out
}

pub fn generate_csv() -> String {
    let mut out = String::new();

    for row in 0..2_000 {
        if row > 0 {
            out.push('\n');
        }

        for col in 0..8 {
            if col > 0 {
                out.push(',');
            }

            let value = ((row * 13 + col * 7) as f64 / 10.0) - 250.0;
            out.push_str(&format!("{value:.3}"));
        }
    }

    out.push('\n');
    out
}

pub fn generate_ini() -> String {
    let mut out = String::new();

    for section in 0..250 {
        out.push_str(&format!("[section_{section}]\n"));

        for key in 0..8 {
            out.push_str(&format!(
                "key_{key}=path/section_{section}/value_{key}\n"
            ));
        }

        out.push('\n');
    }

    out
}

pub fn generate_http() -> String {
    const METHODS: [&str; 4] = ["GET", "POST", "PUT", "DELETE"];
    let mut out = String::new();

    for request in 0..300 {
        let method = METHODS[request % METHODS.len()];
        let version = if request % 3 == 0 { "1.1" } else { "1.0" };

        out.push_str(&format!(
            "{method} /api/v1/items/{request}?page={page} HTTP/{version}\r\n",
            page = request % 17
        ));
        out.push_str("Host: bench.example\r\n");
        out.push_str(&format!("User-Agent: faputa-bench-{request}\r\n"));
        out.push_str(&format!("X-Trace: trace_{request}\r\n"));
        out.push_str("Accept: application/json\r\n");
        out.push_str("\r\n");
    }

    out
}
