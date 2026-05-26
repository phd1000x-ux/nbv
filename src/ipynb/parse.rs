use std::io::Read;
use std::path::Path;

use crate::ipynb::model::Notebook;

pub fn from_str(s: &str) -> Result<Notebook, serde_json::Error> {
    serde_json::from_str(s)
}

pub fn from_reader<R: Read>(r: R) -> Result<Notebook, serde_json::Error> {
    serde_json::from_reader(r)
}

pub fn from_path<P: AsRef<Path>>(p: P) -> std::io::Result<Result<Notebook, serde_json::Error>> {
    let file = std::fs::File::open(p)?;
    // A directory opens cleanly but reading it yields an opaque IO error that
    // serde would surface as a JSON parse failure. Reject it as an IO error so
    // the caller can report it as such instead of "failed to parse".
    if file.metadata()?.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "is a directory, not a notebook file",
        ));
    }
    Ok(from_reader(std::io::BufReader::new(file)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_parses_minimal() {
        let nb = from_str(r#"{"cells":[],"metadata":{},"nbformat":4,"nbformat_minor":5}"#).unwrap();
        assert!(nb.cells.is_empty());
    }

    #[test]
    fn from_str_invalid_json_errors() {
        let r = from_str("not-json");
        assert!(r.is_err());
    }

    #[test]
    fn from_path_reads_file() {
        let tmp = std::env::temp_dir().join("nbv_test_parser.ipynb");
        std::fs::write(&tmp, r#"{"cells":[{"cell_type":"raw","source":"x","metadata":{}}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#).unwrap();
        let nb = from_path(&tmp).unwrap().unwrap();
        assert_eq!(nb.cells.len(), 1);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn from_path_missing_file_returns_io_error() {
        let r = from_path("/definitely/does/not/exist.ipynb");
        assert!(r.is_err());
    }

    #[test]
    fn from_path_directory_is_io_error_not_parse_error() {
        // A directory opens fine but isn't a notebook. Surface it as an IO error
        // (outer Err) so the CLI reports exit 1, not a JSON parse failure (exit 3).
        let r = from_path(std::env::temp_dir());
        assert!(
            r.is_err(),
            "a directory should be an IO error, not Ok(Err(parse))"
        );
    }
}
