//! FAT32-safe filename rules and Deluge-relative path helpers.

const ILLEGAL: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Sanitize a string for use as a FAT32 filename. Strips illegal chars, control codes,
/// trailing dots and spaces. Truncates to 200 chars (FAT32 LFN allows 255 but we keep margin).
pub fn sanitize_filename(s: &str) -> String {
    let mut out: String = s
        .chars()
        .filter(|c| !c.is_control() && !ILLEGAL.contains(c))
        .collect();
    out = out.trim_end_matches(|c: char| c == '.' || c == ' ').to_string();
    if out.is_empty() {
        out = "Untitled".to_string();
    }
    if out.len() > 200 {
        out.truncate(200);
    }
    out
}

/// Compose a kit XML filename: uppercase, `.XML` extension.
pub fn kit_xml_filename(stem: &str) -> String {
    format!("{}.XML", sanitize_filename(stem).to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sanitizes_basic() {
        assert_eq!(sanitize_filename("hello/world?"), "helloworld");
        assert_eq!(sanitize_filename(""), "Untitled");
        assert_eq!(sanitize_filename("trailing... "), "trailing");
    }
    #[test]
    fn kit_filename_uppercase_xml() {
        assert_eq!(kit_xml_filename("mykit"), "MYKIT.XML");
    }
}
