use std::path::Path;

use nu_engine::CallExt;
use nu_protocol::{engine::Command, Example, ShellError, Signature, Span, SyntaxShape, Value};

use super::PathSubcommandArguments;

struct Arguments {
    columns: Option<Vec<String>>,
}

impl PathSubcommandArguments for Arguments {
    fn get_columns(&self) -> Option<Vec<String>> {
        self.columns.clone()
    }
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "path type"
    }

    fn signature(&self) -> Signature {
        Signature::build("path type").named(
            "columns",
            SyntaxShape::Table,
            "Optionally operate by column path",
            Some('c'),
        )
    }

    fn usage(&self) -> &str {
        "Get the type of the object a path refers to (e.g., file, dir, symlink)"
    }

    fn run(
        &self,
        engine_state: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        call: &nu_protocol::ast::Call,
        input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments {
            columns: call.get_flag(engine_state, stack, "columns")?,
        };

        input.map(
            move |value| super::operate(&r#type, &args, value, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Show type of a filepath",
                example: "'.' | path type",
                result: Some(Value::test_string("Dir")),
            },
            Example {
                description: "Show type of a filepath in a column",
                example: "ls | path type -c [ name ]",
                result: None,
            },
        ]
    }
}

fn r#type(path: &Path, span: Span, _: &Arguments) -> Value {
    let meta = std::fs::symlink_metadata(path);

    Value::string(
        match &meta {
            Ok(data) => get_file_type(data),
            Err(_) => "",
        },
        span,
    )
}

fn get_file_type(md: &std::fs::Metadata) -> &str {
    let ft = md.file_type();
    let mut file_type = "Unknown";
    if ft.is_dir() {
        file_type = "Dir";
    } else if ft.is_file() {
        file_type = "File";
    } else if ft.is_symlink() {
        file_type = "Symlink";
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt;
            if ft.is_block_device() {
                file_type = "Block device";
            } else if ft.is_char_device() {
                file_type = "Char device";
            } else if ft.is_fifo() {
                file_type = "Pipe";
            } else if ft.is_socket() {
                file_type = "Socket";
            }
        }
    }
    file_type
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
