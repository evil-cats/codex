//! Общие операции над строками результатов инструментов с сохранением исходных
//! окончаний.

/// Разбивает текст на последовательные строки, сохраняя завершающий `\n` в каждой
/// строке, где он присутствовал в исходном содержимом.
pub(crate) fn split_lines_preserving_endings(content: &str) -> Vec<&str> {
    if content.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut start = 0;
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            lines.push(&content[start..idx + ch.len_utf8()]);
            start = idx + ch.len_utf8();
        }
    }
    if start < content.len() {
        lines.push(&content[start..]);
    }
    lines
}
