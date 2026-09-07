//! Загружает файлы model- и developer-инструкций в effective runtime-строки.
//!
//! Обычный config и agent role используют один контракт порядка, trim и ошибок,
//! чтобы способ выбора профиля не менял содержимое model-visible инструкций.

use codex_exec_server::ExecutorFileSystem;
use codex_exec_server::ReadFileOptions;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_path_uri::PathUri;

/// Отклоняет неоднозначное сочетание одиночного и множественного model-источников.
pub(crate) fn validate_model_instruction_file_settings(
    model_instructions_file: Option<&AbsolutePathBuf>,
    model_instructions_files: &[AbsolutePathBuf],
) -> std::io::Result<()> {
    if model_instructions_file.is_some() && !model_instructions_files.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "`model_instructions_file` and `model_instructions_files` cannot both be set",
        ));
    }

    Ok(())
}

/// Читает model-источники в порядке объявления и разделяет их одной пустой строкой.
pub(crate) async fn load_model_instructions(
    fs: &dyn ExecutorFileSystem,
    model_instructions_file: Option<&AbsolutePathBuf>,
    model_instructions_files: &[AbsolutePathBuf],
) -> std::io::Result<Option<String>> {
    if model_instructions_files.is_empty() {
        return read_non_empty_file(fs, model_instructions_file, "model instructions file").await;
    }

    let mut sections = Vec::with_capacity(model_instructions_files.len());
    for path in model_instructions_files {
        let section = read_non_empty_file(fs, Some(path), "model instructions file")
            .await?
            .expect("a provided model instructions path must produce a section");
        sections.push(section);
    }
    Ok(Some(sections.join("\n\n")))
}

/// Соединяет optional inline developer-секцию с упорядоченными непустыми файлами.
pub(crate) async fn load_developer_instructions(
    fs: &dyn ExecutorFileSystem,
    inline_developer_instructions: Option<&str>,
    developer_instructions_files: &[AbsolutePathBuf],
    startup_warnings: &mut Vec<String>,
) -> std::io::Result<Option<String>> {
    let mut sections = Vec::new();
    if let Some(inline_developer_instructions) = inline_developer_instructions {
        let inline_developer_instructions = inline_developer_instructions.trim();
        if !inline_developer_instructions.is_empty() {
            sections.push(inline_developer_instructions.to_string());
        }
    }

    for path in developer_instructions_files {
        let path_uri = PathUri::from_abs_path(path);
        let contents = fs
            .read_file_text(&path_uri, ReadFileOptions::default(), /*sandbox*/ None)
            .await
            .map_err(|err| {
                std::io::Error::new(
                    err.kind(),
                    format!(
                        "failed to read developer instructions file {}: {err}",
                        path.display()
                    ),
                )
            })?;
        let contents = contents.trim();
        if contents.is_empty() {
            startup_warnings.push(format!(
                "developer instructions file is empty: {}",
                path.display()
            ));
        } else {
            sections.push(contents.to_string());
        }
    }

    Ok((!sections.is_empty()).then(|| sections.join("\n\n")))
}

/// Читает обязательный текстовый источник, сохраняя исходный вид I/O-ошибки.
pub(crate) async fn read_non_empty_file(
    fs: &dyn ExecutorFileSystem,
    path: Option<&AbsolutePathBuf>,
    context: &str,
) -> std::io::Result<Option<String>> {
    let Some(path) = path else {
        return Ok(None);
    };

    let path_uri = PathUri::from_abs_path(path);
    let contents = fs
        .read_file_text(&path_uri, ReadFileOptions::default(), /*sandbox*/ None)
        .await
        .map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!("failed to read {context} {}: {err}", path.display()),
            )
        })?;

    let contents = contents.trim().to_string();
    if contents.is_empty() {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{context} is empty: {}", path.display()),
        ))
    } else {
        Ok(Some(contents))
    }
}
