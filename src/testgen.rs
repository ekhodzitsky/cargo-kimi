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
            Op::Add => "Self(0)",
            Op::Sub => "Self(0)",
            Op::Mul => "Self(1)",
        }
    }

    pub fn has_commutativity(&self) -> bool {
        matches!(self, Op::Add | Op::Mul)
    }

    pub fn has_associativity(&self) -> bool {
        matches!(self, Op::Add | Op::Mul)
    }
}

#[derive(Debug, Clone)]
pub struct TestCase {
    pub type_name: String,
    pub inner_type: String,
    pub ops: Vec<Op>,
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
    let mut impls: HashMap<String, HashSet<Op>> = HashMap::new();

    let struct_re = Regex::new(r"^\s*pub\s+struct\s+(\w+)\s*\(\s*(\w+)\s*\)").unwrap();
    let impl_add_re = Regex::new(r"impl\s+(std::ops::)?Add\s+(for\s+)?(\w+)").unwrap();
    let impl_sub_re = Regex::new(r"impl\s+(std::ops::)?Sub\s+(for\s+)?(\w+)").unwrap();
    let impl_mul_re = Regex::new(r"impl\s+(std::ops::)?Mul\s+(for\s+)?(\w+)").unwrap();

    for entry in walkdir::WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
    {
        let content = std::fs::read_to_string(entry.path())?;
        for line in content.lines() {
            if let Some(cap) = struct_re.captures(line) {
                let name = cap[1].to_string();
                let inner = cap[2].to_string();
                newtypes.insert(
                    name.clone(),
                    Newtype {
                        name,
                        inner_type: inner,
                    },
                );
            }
            if let Some(cap) = impl_add_re.captures(line) {
                let name = cap[3].to_string();
                impls.entry(name).or_default().insert(Op::Add);
            }
            if let Some(cap) = impl_sub_re.captures(line) {
                let name = cap[3].to_string();
                impls.entry(name).or_default().insert(Op::Sub);
            }
            if let Some(cap) = impl_mul_re.captures(line) {
                let name = cap[3].to_string();
                impls.entry(name).or_default().insert(Op::Mul);
            }
        }
    }

    let mut test_cases = Vec::new();
    for (type_name, ops) in impls {
        if let Some(newtype) = newtypes.get(&type_name) {
            test_cases.push(TestCase {
                type_name: type_name.clone(),
                inner_type: newtype.inner_type.clone(),
                ops: ops.into_iter().collect(),
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
        for op in &case.ops {
            let type_name = &case.type_name;
            let op_str = op.as_str().to_lowercase();
            let method = op.method_name();
            let sym = op.symbol();

            if op.has_associativity() {
                output.push_str(&format!(
                    "    proptest! {{\n"
                ));
                output.push_str(&format!(
                    "        #[test]\n"
                ));
                output.push_str(&format!(
                    "        fn {type_name_snake}_{op_str}_associativity(a in any::<{inner_type}>().prop_map({type_name}), b in any::<{inner_type}>().prop_map({type_name}), c in any::<{inner_type}>().prop_map({type_name})) {{\n",
                    type_name_snake = type_name_snake,
                    inner_type = case.inner_type
                ));
                output.push_str(&format!(
                    "            prop_assert_eq!(a {sym} (b {sym} c), (a {sym} b) {sym} c);\n"
                ));
                output.push_str(&format!(
                    "        }}\n"
                ));
                output.push_str(&format!(
                    "    }}\n\n"
                ));
            }

            if op.has_commutativity() {
                output.push_str(&format!(
                    "    proptest! {{\n"
                ));
                output.push_str(&format!(
                    "        #[test]\n"
                ));
                output.push_str(&format!(
                    "        fn {type_name_snake}_{op_str}_commutativity(a in any::<{inner_type}>().prop_map({type_name}), b in any::<{inner_type}>().prop_map({type_name})) {{\n",
                    type_name_snake = type_name_snake,
                    inner_type = case.inner_type
                ));
                output.push_str(&format!(
                    "            prop_assert_eq!(a {sym} b, b {sym} a);\n"
                ));
                output.push_str(&format!(
                    "        }}\n"
                ));
                output.push_str(&format!(
                    "    }}\n\n"
                ));
            }

            // Identity
            let identity = op.identity_value();
            output.push_str(&format!(
                "    proptest! {{\n"
            ));
            output.push_str(&format!(
                "        #[test]\n"
            ));
            output.push_str(&format!(
                "        fn {type_name_snake}_{op_str}_identity(a in any::<{inner_type}>().prop_map({type_name})) {{\n",
                type_name_snake = type_name_snake,
                inner_type = case.inner_type
            ));
            output.push_str(&format!(
                "            prop_assert_eq!(a {sym} {type_name}{}, a);\n",
                &identity[4..]  // strip "Self" prefix, e.g. "(0)" or "(1)"
            ));
            output.push_str(&format!(
                "        }}\n"
            ));
            output.push_str(&format!(
                "    }}\n\n"
            ));
        }
    }

    output.push_str("}\n");
    output
}

fn read_crate_name(manifest_dir: &Path) -> Option<String> {
    let cargo_toml = manifest_dir.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("name") && line.contains('=') {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let name = parts[1].trim().trim_matches('"').trim_matches('\'');
                return Some(name.to_string());
            }
        }
    }
    None
}

pub fn write_tests(src_dir: &Path, output_path: Option<&Path>) -> anyhow::Result<()> {
    let test_cases = scan_project(src_dir)?;

    if test_cases.is_empty() {
        println!("⚠ No newtypes with arithmetic impls found. Nothing to generate.");
        return Ok(());
    }

    println!("Found {} type(s) with arithmetic operations:", test_cases.len());
    for case in &test_cases {
        let ops: Vec<String> = case.ops.iter().map(|o| o.as_str().to_string()).collect();
        println!("  {}: {}", case.type_name, ops.join(", "));
    }

    let manifest_dir = src_dir.parent().unwrap_or(src_dir);
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
