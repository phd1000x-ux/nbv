use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Notebook {
    pub cells: Vec<Cell>,
    #[serde(default)]
    pub metadata: NotebookMetadata,
}

#[derive(Debug, Default, Deserialize)]
pub struct NotebookMetadata {
    pub kernelspec: Option<KernelSpec>,
    pub language_info: Option<LanguageInfo>,
}

#[derive(Debug, Deserialize)]
pub struct KernelSpec {
    pub name: String,
    pub language: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LanguageInfo {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "cell_type", rename_all = "lowercase")]
pub enum Cell {
    Code {
        #[serde(deserialize_with = "string_or_array")]
        source: String,
        #[serde(default)]
        outputs: Vec<Output>,
        #[serde(default)]
        execution_count: Option<u64>,
    },
    Markdown {
        #[serde(deserialize_with = "string_or_array")]
        source: String,
    },
    Raw {
        #[serde(deserialize_with = "string_or_array")]
        source: String,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "output_type", rename_all = "snake_case")]
pub enum Output {
    Stream {
        name: StreamName,
        #[serde(deserialize_with = "string_or_array")]
        text: String,
    },
    ExecuteResult {
        data: MimeBundle,
        #[serde(default)]
        execution_count: Option<u64>,
    },
    DisplayData {
        data: MimeBundle,
    },
    Error {
        ename: String,
        evalue: String,
        #[serde(default)]
        traceback: Vec<String>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamName {
    Stdout,
    Stderr,
}

#[derive(Debug, Default)]
pub struct MimeBundle {
    pub text_plain: Option<String>,
    pub text_html: Option<String>,
    pub image_png: Option<String>,
    pub other: HashMap<String, serde_json::Value>,
}

impl<'de> Deserialize<'de> for MimeBundle {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut raw: HashMap<String, serde_json::Value> = HashMap::deserialize(d)?;
        let text_plain = raw.remove("text/plain").and_then(value_to_string);
        let text_html = raw.remove("text/html").and_then(value_to_string);
        let image_png = raw.remove("image/png").and_then(|v| match v {
            serde_json::Value::String(s) => Some(s),
            _ => None,
        });
        Ok(MimeBundle {
            text_plain,
            text_html,
            image_png,
            other: raw,
        })
    }
}

fn value_to_string(v: serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s),
        serde_json::Value::Array(arr) => {
            let mut s = String::new();
            for item in arr {
                if let serde_json::Value::String(x) = item {
                    s.push_str(&x);
                }
            }
            Some(s)
        }
        _ => None,
    }
}

fn string_or_array<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let v = serde_json::Value::deserialize(d)?;
    value_to_string(v).ok_or_else(|| D::Error::custom("source must be string or array of strings"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_notebook() {
        let json = r#"{"cells":[],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        assert_eq!(nb.cells.len(), 0);
    }

    #[test]
    fn parses_code_cell_with_string_source() {
        let json = r#"{"cells":[{"cell_type":"code","source":"print(1)","outputs":[],"execution_count":1,"metadata":{}}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        match &nb.cells[0] {
            Cell::Code {
                source,
                outputs,
                execution_count,
            } => {
                assert_eq!(source, "print(1)");
                assert!(outputs.is_empty());
                assert_eq!(*execution_count, Some(1));
            }
            _ => panic!("expected code cell"),
        }
    }

    #[test]
    fn parses_code_cell_with_array_source() {
        // ipynb 스펙은 source를 String 또는 Vec<String>으로 허용
        let json = r#"{"cells":[{"cell_type":"code","source":["a=1\n","b=2"],"outputs":[],"metadata":{}}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        match &nb.cells[0] {
            Cell::Code { source, .. } => assert_eq!(source, "a=1\nb=2"),
            _ => panic!(),
        }
    }

    #[test]
    fn parses_markdown_and_raw_cells() {
        let json = r##"{"cells":[
            {"cell_type":"markdown","source":"# Hello","metadata":{}},
            {"cell_type":"raw","source":"raw text","metadata":{}}
        ],"metadata":{},"nbformat":4,"nbformat_minor":5}"##;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        assert!(matches!(nb.cells[0], Cell::Markdown { .. }));
        assert!(matches!(nb.cells[1], Cell::Raw { .. }));
    }

    #[test]
    fn unknown_cell_type_maps_to_unknown() {
        let json = r#"{"cells":[
            {"cell_type":"futuristic","source":"weird","metadata":{}}
        ],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        assert!(matches!(nb.cells[0], Cell::Unknown));
    }

    #[test]
    fn parses_stream_output() {
        let json = r#"{"cells":[{"cell_type":"code","source":"","metadata":{},"outputs":[
            {"output_type":"stream","name":"stdout","text":"hello\n"}
        ]}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        match &nb.cells[0] {
            Cell::Code { outputs, .. } => match &outputs[0] {
                Output::Stream { name, text } => {
                    assert!(matches!(name, StreamName::Stdout));
                    assert_eq!(text, "hello\n");
                }
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn parses_execute_result_with_mimebundle() {
        let json = r#"{"cells":[{"cell_type":"code","source":"","metadata":{},"outputs":[
            {"output_type":"execute_result","execution_count":2,"data":{"text/plain":"42","image/png":"BASE64DATA","text/html":"<table/>"},"metadata":{}}
        ]}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        match &nb.cells[0] {
            Cell::Code { outputs, .. } => match &outputs[0] {
                Output::ExecuteResult {
                    data,
                    execution_count,
                } => {
                    assert_eq!(data.text_plain.as_deref(), Some("42"));
                    assert_eq!(data.image_png.as_deref(), Some("BASE64DATA"));
                    assert_eq!(data.text_html.as_deref(), Some("<table/>"));
                    assert_eq!(*execution_count, Some(2));
                }
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn parses_display_data_output() {
        let json = r#"{"cells":[{"cell_type":"code","source":"","metadata":{},"outputs":[
            {"output_type":"display_data","data":{"text/plain":"<Figure>"},"metadata":{}}
        ]}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        match &nb.cells[0] {
            Cell::Code { outputs, .. } => match &outputs[0] {
                Output::DisplayData { data } => {
                    assert_eq!(data.text_plain.as_deref(), Some("<Figure>"));
                }
                _ => panic!("expected DisplayData"),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn parses_error_output() {
        let json = r#"{"cells":[{"cell_type":"code","source":"","metadata":{},"outputs":[
            {"output_type":"error","ename":"ValueError","evalue":"bad","traceback":["line1","line2"]}
        ]}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        match &nb.cells[0] {
            Cell::Code { outputs, .. } => match &outputs[0] {
                Output::Error {
                    ename,
                    evalue,
                    traceback,
                } => {
                    assert_eq!(ename, "ValueError");
                    assert_eq!(evalue, "bad");
                    assert_eq!(traceback.len(), 2);
                }
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn parses_notebook_metadata_kernelspec() {
        let json = r#"{"cells":[],"metadata":{"kernelspec":{"name":"python3","language":"python","display_name":"Python 3"}},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        let ks = nb.metadata.kernelspec.as_ref().unwrap();
        assert_eq!(ks.language.as_deref(), Some("python"));
    }
}
