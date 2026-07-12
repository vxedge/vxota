//! Minimal Prometheus text-format rendering helpers (same minimal shape as
//! vxagents' — hand-rendered on purpose, not a dependency on a metrics
//! framework, kept as a small standalone copy since each vxedge repo is an
//! independently versioned deliverable).

/// Render one metric line with optional labels.
pub fn line(out: &mut String, name: &str, labels: &[(&str, &str)], value: f64) {
    out.push_str(name);
    if !labels.is_empty() {
        out.push('{');
        for (i, (k, v)) in labels.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(k);
            out.push_str("=\"");
            out.push_str(v);
            out.push('"');
        }
        out.push('}');
    }
    out.push(' ');
    out.push_str(&format!("{value}"));
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_labels() {
        let mut s = String::new();
        line(
            &mut s,
            "vxota_deployments_total",
            &[("status", "success")],
            3.0,
        );
        assert_eq!(s, "vxota_deployments_total{status=\"success\"} 3\n");
    }
}
