use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Newtype {
    pub name: String,
    pub inner_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Op {
    Add,
    Sub,
    Mul,
}

impl Op {
    pub fn as_str(&self) -> &'static str {
        match self {
            Op::Add => "Add",
            Op::Sub => "Sub",
            Op::Mul => "Mul",
        }
    }

    pub fn method_name(&self) -> &'static str {
        match self {
            Op::Add => "add",
            Op::Sub => "sub",
            Op::Mul => "mul",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Op::Add => "+",
            Op::Sub => "-",
            Op::Mul => "*",
        }
    }

    pub fn identity_value(&self) -> &'static str {
        match self {
            Op::Add => "(0)",
            Op::Sub => "(0)",
            Op::Mul => "(1)",
        }
    }

    pub fn has_commutativity(&self) -> bool {
        matches!(self, Op::Add | Op::Mul)
    }

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

pub fn scan_project(src_dir: &Path) -> anyhow::Result<Vec<TestCase>> {
    let mut newtypes: HashMap<String, Newtype> = HashMap::new();
    let mut traits: HashMap<String, Traits> = HashMap::new();

    let struct_re = Regex::new(r"^\s*pub\s+struct\s+(\w+)\s*\(\s*(\w+)\s*\)").unwrap();
    let impl_add_re = Regex::new(r"impl\s+(std::ops::)?Add\s+(for\s+)?(\w+)").unwrap();
    let impl_sub_re = Regex::new(r"impl\s+(std::ops::)?Sub\s+(for\s+)?(\w+)").unwrap();
    let impl_mul_re = Regex::new(r"impl\s+(std::ops::)?Mul\s+(for\s+)?(\w+)").unwrap();
    let impl_ord_re = Regex::new(r"impl\s+Ord\s+(for\s+)?(\w+)").unwrap();
    let impl_eq_re = Regex::new(r"impl\s+(Partial)?Eq\s+(for\s+)?(\w+)").unwrap();
    let impl_clone_re = Regex::new(r"impl\s+Clone\s+(for\s+)?(\w+)").unwrap();
    let derive_re = Regex::new(r"#\[derive\([^)]*(\bOrd\b|\bEq\b|\bClone\b|\bPartialOrd\b|\bPartialEq\b)[^)]*\)\]").unwrap();

    for entry in walkdir::WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
    {
        let content = std::fs::read_to_string(entry.path())?;
        let mut pending_derive: Option<String> = None;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("#[derive(") {
                pending_derive = Some(trimmed.to_string());
                continue;
            }
            if let Some(cap) = struct_re.captures(line) {
                let name = cap[1].to_string();
                let inner = cap[2].to_string();
                newtypes.insert(
                    name.clone(),
                    Newtype { name: name.clone(), inner_type: inner },
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

            if let Some(cap) = impl_add_re.captures(line) {
                traits.entry(cap[3].to_string()).or_default().ops.push(Op::Add);
            }
            if let Some(cap) = impl_sub_re.captures(line) {
                traits.entry(cap[3].to_string()).or_default().ops.push(Op::Sub);
            }
            if let Some(cap) = impl_mul_re.captures(line) {
                traits.entry(cap[3].to_string()).or_default().ops.push(Op::Mul);
            }
            if let Some(cap) = impl_ord_re.captures(line) {
                traits.entry(cap[2].to_string()).or_default().has_ord = true;
            }
            if let Some(cap) = impl_eq_re.captures(line) {
                traits.entry(cap[3].to_string()).or_default().has_eq = true;
            }
            if let Some(cap) = impl_clone_re.captures(line) {
                traits.entry(cap[2].to_string()).or_default().has_clone = true;
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
        for op in &case.traits.ops {
            let op_str = op.as_str().to_lowercase();
            let sym = op.symbol();

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
