use chrono::{DateTime, Utc};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, DataSource, IntoInterruptiblePipelineData, PipelineData, PipelineMetadata,
    ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Clone)]
pub struct Ls;

//NOTE: this is not a real implementation :D. It's just a simple one to test with until we port the real one.
impl Command for Ls {
    fn name(&self) -> &str {
        "ls"
    }

    fn usage(&self) -> &str {
        "List the files in a directory."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("ls")
            .optional(
                "pattern",
                SyntaxShape::GlobPattern,
                "the glob pattern to use",
            )
            .switch("all", "Show hidden files", Some('a'))
            .switch(
                "long",
                "List all available columns for each entry",
                Some('l'),
            )
            .switch(
                "short-names",
                "Only print the file names and not the path",
                Some('s'),
            )
            // .switch(
            //     "du",
            //     "Display the apparent directory size in place of the directory metadata size",
            //     Some('d'),
            // )
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let all = call.has_flag("all");
        let long = call.has_flag("long");
        let short_names = call.has_flag("short-names");

        let call_span = call.head;

        let pattern = if let Some(mut result) =
            call.opt::<Spanned<String>>(engine_state, stack, 0)?
        {
            let path = std::path::Path::new(&result.item);

            if path.is_dir() {
                if permission_denied(&path) {
                    #[cfg(unix)]
                    let error_msg = format!(
                        "The permissions of {:o} do not allow access for this user",
                        path.metadata()
                            .expect("this shouldn't be called since we already know there is a dir")
                            .permissions()
                            .mode()
                            & 0o0777
                    );
                    #[cfg(not(unix))]
                    let error_msg = String::from("Permission denied");
                    return Err(ShellError::SpannedLabeledError(
                        "Permission denied".into(),
                        error_msg,
                        result.span,
                    ));
                }
                if is_empty_dir(&path) {
                    return Ok(PipelineData::new(call_span));
                }

                if path.is_dir() {
                    if !result.item.ends_with(std::path::MAIN_SEPARATOR) {
                        result.item.push(std::path::MAIN_SEPARATOR);
                    }
                    result.item.push('*');
                }
            }

            result.item
        } else {
            "*".into()
        };

        let glob = glob::glob(&pattern).map_err(|err| {
            nu_protocol::ShellError::SpannedLabeledError(
                "Error extracting glob pattern".into(),
                err.to_string(),
                call.head,
            )
        })?;

        let hidden_dir_specified = is_hidden_dir(&pattern);
        let mut hidden_dirs = vec![];

        Ok(glob
            .into_iter()
            .filter_map(move |x| match x {
                Ok(path) => {
                    let metadata = match std::fs::symlink_metadata(&path) {
                        Ok(metadata) => Some(metadata),
                        Err(e) => {
                            if e.kind() == ErrorKind::PermissionDenied
                                || e.kind() == ErrorKind::Other
                            {
                                None
                            } else {
                                return Some(Value::Error {
                                    error: ShellError::IOError(format!("{}", e)),
                                });
                            }
                        }
                    };
                    if path_contains_hidden_folder(&path, &hidden_dirs) {
                        return None;
                    }

                    if !all && !hidden_dir_specified && is_hidden_dir(&path) {
                        if path.is_dir() {
                            hidden_dirs.push(path);
                        }
                        return None;
                    }

                    let entry =
                        dir_entry_dict(&path, metadata.as_ref(), call_span, long, short_names);

                    match entry {
                        Ok(value) => Some(value),
                        Err(err) => Some(Value::Error { error: err }),
                    }
                }
                _ => Some(Value::Nothing { span: call_span }),
            })
            .into_pipeline_data_with_metadata(
                PipelineMetadata {
                    data_source: DataSource::Ls,
                },
                engine_state.ctrlc.clone(),
            ))
    }
}

fn permission_denied(dir: impl AsRef<Path>) -> bool {
    match dir.as_ref().read_dir() {
        Err(e) => matches!(e.kind(), std::io::ErrorKind::PermissionDenied),
        Ok(_) => false,
    }
}

fn is_hidden_dir(dir: impl AsRef<Path>) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        if let Ok(metadata) = dir.as_ref().metadata() {
            let attributes = metadata.file_attributes();
            // https://docs.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
            (attributes & 0x2) != 0
        } else {
            false
        }
    }

    #[cfg(not(windows))]
    {
        dir.as_ref()
            .file_name()
            .map(|name| name.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
    }
}

fn is_empty_dir(dir: impl AsRef<Path>) -> bool {
    match dir.as_ref().read_dir() {
        Err(_) => true,
        Ok(mut s) => s.next().is_none(),
    }
}

fn path_contains_hidden_folder(path: &Path, folders: &[PathBuf]) -> bool {
    let path_str = path.to_str().expect("failed to read path");
    if folders
        .iter()
        .any(|p| path_str.starts_with(&p.to_str().expect("failed to read hidden paths")))
    {
        return true;
    }
    false
}

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};

pub fn get_file_type(md: &std::fs::Metadata) -> &str {
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn dir_entry_dict(
    filename: &std::path::Path,
    metadata: Option<&std::fs::Metadata>,
    span: Span,
    long: bool,
    short_name: bool,
) -> Result<Value, ShellError> {
    let mut cols = vec![];
    let mut vals = vec![];

    let name = if short_name {
        filename.file_name().and_then(|s| s.to_str())
    } else {
        filename.to_str()
    }
    .ok_or_else(|| {
        ShellError::SpannedLabeledError(
            format!("Invalid file name: {:}", filename.to_string_lossy()),
            "invalid file name".into(),
            span,
        )
    })?;

    cols.push("name".into());
    vals.push(Value::String {
        val: name.to_string(),
        span,
    });

    if let Some(md) = metadata {
        cols.push("type".into());
        vals.push(Value::String {
            val: get_file_type(md).to_string(),
            span,
        });
    } else {
        cols.push("type".into());
        vals.push(Value::nothing(span));
    }

    if long {
        cols.push("target".into());
        if let Some(md) = metadata {
            if md.file_type().is_symlink() {
                if let Ok(path_to_link) = filename.read_link() {
                    vals.push(Value::String {
                        val: path_to_link.to_string_lossy().to_string(),
                        span,
                    });
                } else {
                    vals.push(Value::String {
                        val: "Could not obtain target file's path".to_string(),
                        span,
                    });
                }
            } else {
                vals.push(Value::nothing(span));
            }
        }
    }

    if long {
        if let Some(md) = metadata {
            cols.push("readonly".into());
            vals.push(Value::Bool {
                val: md.permissions().readonly(),
                span,
            });

            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let mode = md.permissions().mode();
                cols.push("mode".into());
                vals.push(Value::String {
                    val: umask::Mode::from(mode).to_string(),
                    span,
                });

                let nlinks = md.nlink();
                cols.push("num_links".into());
                vals.push(Value::Int {
                    val: nlinks as i64,
                    span,
                });

                let inode = md.ino();
                cols.push("inode".into());
                vals.push(Value::Int {
                    val: inode as i64,
                    span,
                });

                cols.push("uid".into());
                if let Some(user) = users::get_user_by_uid(md.uid()) {
                    vals.push(Value::String {
                        val: user.name().to_string_lossy().into(),
                        span,
                    });
                } else {
                    vals.push(Value::nothing(span))
                }

                cols.push("group".into());
                if let Some(group) = users::get_group_by_gid(md.gid()) {
                    vals.push(Value::String {
                        val: group.name().to_string_lossy().into(),
                        span,
                    });
                } else {
                    vals.push(Value::nothing(span))
                }
            }
        }
    }

    cols.push("size".to_string());
    if let Some(md) = metadata {
        if md.is_dir() {
            let dir_size: u64 = md.len();

            vals.push(Value::Filesize {
                val: dir_size as i64,
                span,
            });
        } else if md.is_file() {
            vals.push(Value::Filesize {
                val: md.len() as i64,
                span,
            });
        } else if md.file_type().is_symlink() {
            if let Ok(symlink_md) = filename.symlink_metadata() {
                vals.push(Value::Filesize {
                    val: symlink_md.len() as i64,
                    span,
                });
            } else {
                vals.push(Value::nothing(span));
            }
        }
    } else {
        vals.push(Value::nothing(span));
    }

    if let Some(md) = metadata {
        if long {
            cols.push("created".to_string());
            if let Ok(c) = md.created() {
                let utc: DateTime<Utc> = c.into();
                vals.push(Value::Date {
                    val: utc.into(),
                    span,
                });
            } else {
                vals.push(Value::nothing(span));
            }

            cols.push("accessed".to_string());
            if let Ok(a) = md.accessed() {
                let utc: DateTime<Utc> = a.into();
                vals.push(Value::Date {
                    val: utc.into(),
                    span,
                });
            } else {
                vals.push(Value::nothing(span));
            }
        }

        cols.push("modified".to_string());
        if let Ok(m) = md.modified() {
            let utc: DateTime<Utc> = m.into();
            vals.push(Value::Date {
                val: utc.into(),
                span,
            });
        } else {
            vals.push(Value::nothing(span));
        }
    } else {
        if long {
            cols.push("created".to_string());
            vals.push(Value::nothing(span));

            cols.push("accessed".to_string());
            vals.push(Value::nothing(span));
        }

        cols.push("modified".to_string());
        vals.push(Value::nothing(span));
    }

    Ok(Value::Record { cols, vals, span })
}
