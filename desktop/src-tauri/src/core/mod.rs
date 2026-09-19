pub(crate) mod adapter;
pub(crate) mod desktop;
pub(crate) mod drift;
pub(crate) mod hash;
pub(crate) mod iox;
pub(crate) mod jsonkeys;
pub(crate) mod mcp;
pub(crate) mod paths;
pub(crate) mod source;
pub(crate) mod state;

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::Path;

    pub(crate) struct TestDir(tempfile::TempDir);

    impl TestDir {
        pub(crate) fn new(label: &str) -> Self {
            Self(
                tempfile::Builder::new()
                    .prefix(&format!("agent-assistant-{label}-"))
                    .tempdir()
                    .expect("create private test directory"),
            )
        }

        pub(crate) fn path(&self) -> &Path {
            self.0.path()
        }
    }
}
