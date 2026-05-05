fn main() -> anyhow::Result<()> {
    cargo_kimi::cli::run()
}
#[allow(dead_code)]
pub struct MainArgs(pub(crate) Vec<String>);
