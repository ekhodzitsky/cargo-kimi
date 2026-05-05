// kimi:score-ignore=unwrap
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

static STRUCT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*pub\s+struct\s+(\w+)\s*\(\s*(\w+)\s*\)").unwrap());
static IMPL_ADD_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"impl\s+(std::ops::)?Add\s+(for\s+)?(\w+)").unwrap());
static IMPL_SUB_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"impl\s+(std::ops::)?Sub\s+(for\s+)?(\w+)").unwrap());
static IMPL_MUL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"impl\s+(std::ops::)?Mul\s+(for\s+)?(\w+)").unwrap());
static IMPL_ORD_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"impl\s+Ord\s+(for\s+)?(\w+)").unwrap());
static IMPL_EQ_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"impl\s+(Partial)?Eq\s+(for\s+)?(\w+)").unwrap());
static IMPL_CLONE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"impl\s+Clone\s+(for\s+)?(\w+)").unwrap());

#[derive(Debug, Clone)]
pub struct Newtype {
    pub inner_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Op {
    Add,
    Sub,
    Mul,
}

impl Op {
    /// { true }
    /// fn as_str(&self) -> &'static str
    /// { result ∈ {"Add", "Sub", "Mul"} }
    pub fn as_str(&self) -> &'static str {
        match self {
            Op::Add => "Add",
            Op::Sub => "Sub",
            Op::Mul => "Mul",
        }
    }

    /// { true }
    /// fn symbol(&self) -> &'static str
    /// { result ∈ {"+", "-", "*"} }
    pub fn symbol(&self) -> &'static str {
        match self {
            Op::Add => "+",
            Op::Sub => "-",
            Op::Mul => "*",
        }
    }

    /// { true }
    /// fn identity_value(&self) -> &'static str
    /// { result ∈ {"(0)", "(1)"} }
    pub fn identity_value(&self) -> &'static str {
        match self {
            Op::Add => "(0)",
            Op::Sub => "(0)",
            Op::Mul => "(1)",
        }
    }

    /// { true }
    /// fn wrapping_method(&self) -> &'static str
    /// { result ∈ {"wrapping_add", "wrapping_sub", "wrapping_mul"} }
    pub fn wrapping_method(&self) -> &'static str {
        match self {
            Op::Add => "wrapping_add",
            Op::Sub => "wrapping_sub",
            Op::Mul => "wrapping_mul",
        }
    }

    /// { true }
    /// fn has_commutativity(&self) -> bool
    /// { result == (self == Add || self == Mul) }
    pub fn has_commutativity(&self) -> bool {
        matches!(self, Op::Add | Op::Mul)
    }

    /// { true }
    /// fn has_associativity(&self) -> bool
    /// { result == (self == Add || self == Mul) }
    pub fn has_associativity(&self) -> bool {
        matches!(self, Op::Add | Op::Mul)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Traits {
    pub ops: Vec<Op>,
    pub has_ord: bool,
    pub has_eq: bool,
    pub has_clone: bool,
}

#[derive(Debug, Clone)]
pub struct TestCase {
    pub type_name: String,
    pub inner_type: String,
    pub traits: Traits,
}

/// { t is a valid Rust primitive type name }
/// fn is_integral_type(t: &str) -> bool
/// { result == (t is an integer primitive) }
fn is_integral_type(t: &str) -> bool {
    matches!(t, "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64" | "i128" | "isize")
}

/// { s is a valid ASCII identifier }
/// fn to_snake_case(s: &str) -> String
/// { result == s converted to snake_case }
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// { src_dir exists and is readable }
/// pub fn scan_project(src_dir: &Path) -> anyhow::Result<Vec<TestCase>>
/// { result contains all newtypes with arithmetic traits found in src_dir }
pub fn scan_project(src_dir: &Path) -> anyhow::Result<Vec<TestCase>> {
    let mut newtypes: HashMap<String, Newtype> = HashMap::new();
    let mut traits: HashMap<String, Traits> = HashMap::new();

    for entry_result in walkdir::WalkDir::new(src_dir).follow_links(false) {
        let entry = match entry_result {
            Ok(e) => e,
            Err(err) => {
                eprintln!("Warning: {}", err);
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().map(|ext| ext == "rs").unwrap_or(false) {
            let bytes = match std::fs::read(entry.path()) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("Warning: could not read {}: {}", entry.path().display(), e);
                    continue;
                }
            };
            let content = match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("Warning: skipping non-UTF8 file: {}", entry.path().display());
                    continue;
                }
            };
            let mut pending_derive: Option<String> = None;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("#[derive(") {
                    pending_derive = Some(trimmed.to_string());
                    continue;
                }
                if let Some(cap) = STRUCT_RE.captures(line) {
                    let name = cap[1].to_string();
                    let inner = cap[2].to_string();
                    newtypes.insert(
                        name.clone(),
                        Newtype { inner_type: inner },
                    );
                    if let Some(ref derive) = pending_derive {
                        if derive.contains("Ord") { traits.entry(name.clone()).or_default().has_ord = true; }
                        if derive.contains("Eq") || derive.contains("PartialEq") { traits.entry(name.clone()).or_default().has_eq = true; }
                        if derive.contains("Clone") { traits.entry(name.clone()).or_default().has_clone = true; }
                    }
                    pending_derive = None;
                    continue;
                }
                pending_derive = None;

                if let Some(cap) = IMPL_ADD_RE.captures(line) {
                    traits.entry(cap[3].to_string()).or_default().ops.push(Op::Add);
                }
                if let Some(cap) = IMPL_SUB_RE.captures(line) {
                    traits.entry(cap[3].to_string()).or_default().ops.push(Op::Sub);
                }
                if let Some(cap) = IMPL_MUL_RE.captures(line) {
                    traits.entry(cap[3].to_string()).or_default().ops.push(Op::Mul);
                }
                if let Some(cap) = IMPL_ORD_RE.captures(line) {
                    traits.entry(cap[2].to_string()).or_default().has_ord = true;
                }
                if let Some(cap) = IMPL_EQ_RE.captures(line) {
                    traits.entry(cap[3].to_string()).or_default().has_eq = true;
                }
                if let Some(cap) = IMPL_CLONE_RE.captures(line) {
                    traits.entry(cap[2].to_string()).or_default().has_clone = true;
                }
            }
        }
    }

    let mut test_cases = Vec::new();
    for (type_name, t) in traits {
        if let Some(newtype) = newtypes.get(&type_name) {
            test_cases.push(TestCase {
                type_name: type_name.clone(),
                inner_type: newtype.inner_type.clone(),
                traits: t,
            });
        }
    }

    test_cases.sort_by(|a, b| a.type_name.cmp(&b.type_name));
    Ok(test_cases)
}

    /// { test_cases are valid scanned newtype definitions }
    /// pub fn generate_tests(test_cases: &[TestCase]) -> String
    /// { result is valid Rust source code for proptest property tests }
pub fn generate_tests(test_cases: &[TestCase]) -> String {
    let mut output = String::new();
    output.push_str("#[cfg(test)]\n");
    output.push_str("mod property_tests {\n");
    for case in test_cases {
        output.push_str(&format!("    use crate::{};\n", case.type_name));
    }
    output.push_str("    use proptest::prelude::*;\n");
    output.push_str("    use proptest::strategy::Strategy;\n\n");

    for case in test_cases {
        let type_name_snake = to_snake_case(&case.type_name);
        let type_name = &case.type_name;
        let inner = &case.inner_type;

        // Arithmetic tests
        let integral = is_integral_type(&case.inner_type);
        for op in &case.traits.ops {
            let op_str = op.as_str().to_lowercase();
            let sym = op.symbol();

            if integral {
                let wrap = op.wrapping_method();
                if op.has_associativity() {
                    output.push_str("    proptest! {\n");
                    output.push_str("        #[test]\n");
                    output.push_str(&format!(
                        "        fn {type_name_snake}_{op_str}_associativity(a in any::<{inner}>().prop_map({type_name}), b in any::<{inner}>().prop_map({type_name}), c in any::<{inner}>().prop_map({type_name})) {{\n"
                    ));
                    output.push_str(&format!(
                        "            prop_assert_eq!(a.{wrap}(b.{wrap}(c)), (a.{wrap}(b)).{wrap}(c));\n"
                    ));
                    output.push_str("        }\n");
                    output.push_str("    }\n\n");
                }

                if op.has_commutativity() {
                    output.push_str("    proptest! {\n");
                    output.push_str("        #[test]\n");
                    output.push_str(&format!(
                        "        fn {type_name_snake}_{op_str}_commutativity(a in any::<{inner}>().prop_map({type_name}), b in any::<{inner}>().prop_map({type_name})) {{\n"
                    ));
                    output.push_str(&format!(
                        "            prop_assert_eq!(a.{wrap}(b), b.{wrap}(a));\n"
                    ));
                    output.push_str("        }\n");
                    output.push_str("    }\n\n");
                }

                let identity = op.identity_value();
                output.push_str("    proptest! {\n");
                output.push_str("        #[test]\n");
                output.push_str(&format!(
                    "        fn {type_name_snake}_{op_str}_identity(a in any::<{inner}>().prop_map({type_name})) {{\n"
                ));
                output.push_str(&format!(
                    "            prop_assert_eq!(a.{wrap}({type_name}{identity}), a);\n"
                ));
                output.push_str("        }\n");
                output.push_str("    }\n\n");
            } else {
                if op.has_associativity() {
                    output.push_str("    proptest! {\n");
                    output.push_str("        #[test]\n");
                    output.push_str(&format!(
                        "        fn {type_name_snake}_{op_str}_associativity(a in any::<{inner}>().prop_map({type_name}), b in any::<{inner}>().prop_map({type_name}), c in any::<{inner}>().prop_map({type_name})) {{\n"
                    ));
                    output.push_str(&format!(
                        "            prop_assert_eq!(a {sym} (b {sym} c), (a {sym} b) {sym} c);\n"
                    ));
                    output.push_str("        }\n");
                    output.push_str("    }\n\n");
                }

                if op.has_commutativity() {
                    output.push_str("    proptest! {\n");
                    output.push_str("        #[test]\n");
                    output.push_str(&format!(
                        "        fn {type_name_snake}_{op_str}_commutativity(a in any::<{inner}>().prop_map({type_name}), b in any::<{inner}>().prop_map({type_name})) {{\n"
                    ));
                    output.push_str(&format!(
                        "            prop_assert_eq!(a {sym} b, b {sym} a);\n"
                    ));
                    output.push_str("        }\n");
                    output.push_str("    }\n\n");
                }

                let identity = op.identity_value();
                output.push_str("    proptest! {\n");
                output.push_str("        #[test]\n");
                output.push_str(&format!(
                    "        fn {type_name_snake}_{op_str}_identity(a in any::<{inner}>().prop_map({type_name})) {{\n"
                ));
                output.push_str(&format!(
                    "            prop_assert_eq!(a {sym} {type_name}{identity}, a);\n"
                ));
                output.push_str("        }\n");
                output.push_str("    }\n\n");
            }
        }

        // Ord tests
        if case.traits.has_ord {
            output.push_str("    proptest! {\n");
            output.push_str("        #[test]\n");
            output.push_str(&format!(
                "        fn {type_name_snake}_ord_transitivity(a in any::<{inner}>().prop_map({type_name}), b in any::<{inner}>().prop_map({type_name}), c in any::<{inner}>().prop_map({type_name})) {{\n"
            ));
            output.push_str("            if a < b && b < c {\n");
            output.push_str("                prop_assert!(a < c);\n");
            output.push_str("            }\n");
            output.push_str("        }\n");
            output.push_str("    }\n\n");

            output.push_str("    proptest! {\n");
            output.push_str("        #[test]\n");
            output.push_str(&format!(
                "        fn {type_name_snake}_ord_antisymmetry(a in any::<{inner}>().prop_map({type_name}), b in any::<{inner}>().prop_map({type_name})) {{\n"
            ));
            output.push_str("            if a <= b && b <= a {\n");
            output.push_str("                prop_assert_eq!(a, b);\n");
            output.push_str("            }\n");
            output.push_str("        }\n");
            output.push_str("    }\n\n");
        }

        // Eq tests
        if case.traits.has_eq {
            output.push_str("    proptest! {\n");
            output.push_str("        #[test]\n");
            output.push_str(&format!(
                "        fn {type_name_snake}_eq_reflexivity(a in any::<{inner}>().prop_map({type_name})) {{\n"
            ));
            output.push_str("            prop_assert_eq!(a, a);\n");
            output.push_str("        }\n");
            output.push_str("    }\n\n");

            output.push_str("    proptest! {\n");
            output.push_str("        #[test]\n");
            output.push_str(&format!(
                "        fn {type_name_snake}_eq_symmetry(a in any::<{inner}>().prop_map({type_name}), b in any::<{inner}>().prop_map({type_name})) {{\n"
            ));
            output.push_str("            prop_assert_eq!(a == b, b == a);\n");
            output.push_str("        }\n");
            output.push_str("    }\n\n");
        }

        // Clone tests
        if case.traits.has_clone {
            output.push_str("    proptest! {\n");
            output.push_str("        #[test]\n");
            output.push_str(&format!(
                "        fn {type_name_snake}_clone_eq_original(a in any::<{inner}>().prop_map({type_name})) {{\n"
            ));
            output.push_str("            prop_assert_eq!(a.clone(), a);\n");
            output.push_str("        }\n");
            output.push_str("    }\n\n");
        }
    }

    output.push_str("}\n");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_snake_case_variants() {
        assert_eq!(to_snake_case("FooBar"), "foo_bar");
        assert_eq!(to_snake_case("Foo"), "foo");
        assert_eq!(to_snake_case("foo_bar"), "foo_bar");
        assert_eq!(to_snake_case("A"), "a");
    }

    #[test]
    fn is_integral_type_all_twelve_true() {
        for t in [
            "u8", "u16", "u32", "u64", "u128", "usize",
            "i8", "i16", "i32", "i64", "i128", "isize",
        ] {
            assert!(is_integral_type(t), "{} should be integral", t);
        }
    }

    #[test]
    fn is_integral_type_non_integral_false() {
        for t in ["String", "f32", "bool"] {
            assert!(!is_integral_type(t), "{} should not be integral", t);
        }
    }

    #[test]
    fn op_wrapping_method_variants() {
        assert_eq!(Op::Add.wrapping_method(), "wrapping_add");
        assert_eq!(Op::Sub.wrapping_method(), "wrapping_sub");
        assert_eq!(Op::Mul.wrapping_method(), "wrapping_mul");
    }
}

    /// { src_dir exists and is writable }
    /// pub fn write_tests(src_dir: &Path, output_path: Option<&Path>) -> anyhow::Result<()>
    /// { writes generated property tests to output_path or src_dir/kimi_property_tests.rs }
pub fn write_tests(src_dir: &Path, output_path: Option<&Path>) -> anyhow::Result<()> {
    let test_cases = scan_project(src_dir)?;

    if test_cases.is_empty() {
        println!("⚠ No newtypes with traits found. Nothing to generate.");
        return Ok(());
    }

    println!("Found {} type(s):", test_cases.len());
    for case in &test_cases {
        let mut parts = Vec::new();
        for op in &case.traits.ops {
            parts.push(op.as_str().to_string());
        }
        if case.traits.has_ord { parts.push("Ord".to_string()); }
        if case.traits.has_eq { parts.push("Eq".to_string()); }
        if case.traits.has_clone { parts.push("Clone".to_string()); }
        println!("  {}: {}", case.type_name, parts.join(", "));
    }

    let generated = generate_tests(&test_cases);

    let target = match output_path {
        Some(p) => p.to_path_buf(),
        None => src_dir.join("kimi_property_tests.rs"),
    };

    // Validate output path doesn't escape project root
    if target.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        anyhow::bail!("Output path cannot contain parent directory references (..)");
    }
    let cwd = std::env::current_dir()?.canonicalize()?;
    let abs_target = if target.is_absolute() { target.clone() } else { cwd.join(&target) };
    let mut normalized = std::path::PathBuf::new();
    for comp in abs_target.components() {
        match comp {
            std::path::Component::CurDir => {}
            _ => normalized.push(comp),
        }
    }
    if !normalized.starts_with(&cwd) {
        anyhow::bail!("Output path must be inside the project directory");
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&target, generated)?;
    println!("✓ Generated property tests: {}", target.display());
    println!("\nNext steps:");
    println!("  1. Add to your src/lib.rs or src/main.rs:");
    println!("     #[cfg(test)]");
    println!("     mod kimi_property_tests;");
    println!("  2. Add proptest to [dev-dependencies] in Cargo.toml:");
    println!("     proptest = \"1.0\"");

    Ok(())
}
