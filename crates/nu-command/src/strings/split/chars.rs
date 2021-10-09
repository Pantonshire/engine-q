use nu_protocol::{
    ast::Call,
    engine::{Command, EvaluationContext},
    Example, ShellError, Signature, Span, Type, Value,
};

pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "split chars"
    }

    fn signature(&self) -> Signature {
        Signature::build("split chars")
    }

    fn usage(&self) -> &str {
        "splits a string's characters into separate rows"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Split the string's characters into separate rows",
            example: "'hello' | split chars",
            result: Some(Value::List {
                vals: vec![
                    Value::String {
                        val: "h".into(),
                        span: Span::unknown(),
                    },
                    Value::String {
                        val: "e".into(),
                        span: Span::unknown(),
                    },
                    Value::String {
                        val: "l".into(),
                        span: Span::unknown(),
                    },
                    Value::String {
                        val: "l".into(),
                        span: Span::unknown(),
                    },
                    Value::String {
                        val: "o".into(),
                        span: Span::unknown(),
                    },
                ],
                span: Span::unknown(),
            }),
        }]
    }

    fn run(
        &self,
        _context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        split_chars(call, input)
    }
}

fn split_chars(call: &Call, input: Value) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
    let span = call.head;

    Ok(input.flat_map(span, move |x| split_chars_helper(&x, span)))
}

fn split_chars_helper(v: &Value, name: Span) -> Vec<Value> {
    if let Ok(s) = v.as_string() {
        let v_span = v.span();
        s.chars()
            .collect::<Vec<_>>()
            .into_iter()
            .map(move |x| Value::String {
                val: x.to_string(),
                span: v_span,
            })
            .collect()
    } else {
        vec![Value::Error {
            error: ShellError::PipelineMismatch {
                expected: Type::String,
                expected_span: name,
                origin: v.span(),
            },
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}