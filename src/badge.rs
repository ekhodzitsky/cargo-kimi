/// { score is 0–100 }
/// pub fn generate_svg(score: u32) -> String
/// { returns a shields.io-style SVG badge }
pub fn generate_svg(score: u32) -> String {
    let label = "kimi";
    let value = format!("{}/100", score);

    let (color, text_color) = match score {
        80..=100 => ("#4CAF50", "#ffffff"),
        60..=79 => ("#8BC34A", "#ffffff"),
        40..=59 => ("#FFC107", "#000000"),
        20..=39 => ("#FF9800", "#ffffff"),
        _ => ("#F44336", "#ffffff"),
    };

    let label_width = label.len() * 7 + 12;
    let value_width = value.len() * 7 + 12;
    let total_width = label_width + value_width;
    let label_center = label_width / 2;
    let value_center = label_width + value_width / 2;

    let mut svg = String::new();
    svg.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{}" height="20" role="img" aria-label="{}: {}">
  <title>{}: {}</title>
  <linearGradient id="s" x2="0" y2="100%">
    <stop offset="0" stop-color="#bbb" stop-opacity=".1"/>
    <stop offset="1" stop-opacity=".1"/>
  </linearGradient>
  <clipPath id="r">
    <rect width="{}" height="20" rx="3" fill="#ffffff"/>
  </clipPath>
  <g clip-path="url(#r)">
    <rect width="{}" height="20" fill="#555"/>
    <rect x="{}" width="{}" height="20" fill="{}"/>
    <rect width="{}" height="20" fill="url(#s)"/>
  </g>
  <g fill="{}" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans serif" font-size="11">
    <text x="{}" y="14" fill="#010101" fill-opacity=".3">{}</text>
    <text x="{}" y="13">{}</text>
    <text x="{}" y="14" fill="#010101" fill-opacity=".3">{}</text>
    <text x="{}" y="13">{}</text>
  </g>
</svg>"##,
        total_width, label, value,
        label, value,
        total_width,
        label_width, label_width, value_width, color,
        total_width,
        text_color,
        label_center, label,
        label_center, label,
        value_center, value,
        value_center, value,
    ));
    svg
}

    /// { path is a valid file path }
    /// pub fn write_badge(path: &std::path::Path, score: u32) -> anyhow::Result<()>
    /// { writes SVG badge to disk }
pub fn write_badge(path: &std::path::Path, score: u32) -> anyhow::Result<()> {
    let svg = generate_svg(score);
    std::fs::write(path, svg)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_svg_includes_score() {
        let svg = generate_svg(85);
        assert!(svg.contains("85/100"));
        assert!(svg.contains("kimi"));
        assert!(svg.starts_with("<svg"));
    }

    #[test]
    fn generate_svg_color_ranges() {
        let excellent = generate_svg(90);
        assert!(excellent.contains("#4CAF50")); // green

        let good = generate_svg(70);
        assert!(good.contains("#8BC34A")); // light green

        let warning = generate_svg(50);
        assert!(warning.contains("#FFC107")); // yellow

        let poor = generate_svg(30);
        assert!(poor.contains("#FF9800")); // orange

        let critical = generate_svg(10);
        assert!(critical.contains("#F44336")); // red
    }
}
