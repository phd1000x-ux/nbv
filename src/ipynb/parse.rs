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
}
