use iced_core::clipboard::{Clipboard, Kind as ClipboardKind};

/// Read text for a terminal paste operation.
pub(crate) fn read_paste_text(clipboard: &dyn Clipboard) -> Option<String> {
    #[cfg(target_os = "macos")]
    return read_paste_text_from(clipboard, macos_file_url_path());

    #[cfg(not(target_os = "macos"))]
    clipboard.read(ClipboardKind::Standard)
}

#[cfg(target_os = "macos")]
fn read_paste_text_from(
    clipboard: &dyn Clipboard,
    file_url_path: Option<String>,
) -> Option<String> {
    if file_url_path.is_some() {
        return file_url_path;
    }

    clipboard.read(ClipboardKind::Standard).map(|text| {
        if let Some(path) = file_url_to_posix_path(&text) {
            return path;
        }

        text
    })
}

#[cfg(target_os = "macos")]
fn macos_file_url_path() -> Option<String> {
    use objc2_app_kit::NSPasteboard;

    let pasteboard = unsafe { NSPasteboard::generalPasteboard() };
    macos_file_url_path_from(&pasteboard)
}

#[cfg(target_os = "macos")]
fn macos_file_url_path_from(
    pasteboard: &objc2_app_kit::NSPasteboard,
) -> Option<String> {
    use objc2_app_kit::NSPasteboardTypeFileURL;

    let file_url =
        unsafe { pasteboard.stringForType(NSPasteboardTypeFileURL) }?;

    file_url_to_posix_path(&file_url.to_string())
}

#[cfg(target_os = "macos")]
fn file_url_to_posix_path(value: &str) -> Option<String> {
    use objc2_foundation::{NSString, NSURL};

    let value = NSString::from_str(value);
    let url = unsafe { NSURL::URLWithString(&value) }?;
    let scheme = unsafe { url.scheme()? };
    if !scheme.to_string().eq_ignore_ascii_case("file") {
        return None;
    }

    unsafe { url.path() }.map(|path| path.to_string())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use iced_core::clipboard::Kind as ClipboardKind;
    use objc2_app_kit::{
        NSPasteboard, NSPasteboardTypeFileURL, NSPasteboardTypeString,
    };
    use objc2_foundation::NSString;

    use super::{
        file_url_to_posix_path, macos_file_url_path_from, read_paste_text_from,
    };

    struct TextClipboard {
        value: String,
    }

    impl iced_core::clipboard::Clipboard for TextClipboard {
        fn read(&self, kind: ClipboardKind) -> Option<String> {
            (kind == ClipboardKind::Standard).then(|| self.value.clone())
        }

        fn write(&mut self, _kind: ClipboardKind, _contents: String) {}
    }

    #[test]
    fn file_url_to_posix_path_decodes_file_url() {
        assert_eq!(
            file_url_to_posix_path(
                "file:///Users/example/My%20File%20%E3%81%82.txt"
            )
            .as_deref(),
            Some("/Users/example/My File あ.txt")
        );
    }

    #[test]
    fn file_url_to_posix_path_rejects_non_file_url() {
        assert_eq!(
            file_url_to_posix_path("https://example.com/file.txt"),
            None
        );
    }

    #[test]
    fn macos_pasteboard_prefers_file_url_over_text_representation() {
        let pasteboard = unsafe { NSPasteboard::pasteboardWithUniqueName() };
        let file_url = NSString::from_str(
            "file:///Users/example/My%20File%20%E3%81%82.txt",
        );
        let written = unsafe {
            pasteboard.setString_forType(&file_url, NSPasteboardTypeFileURL)
        };
        let text = NSString::from_str("otty-file-icon.icns");
        let text_written = unsafe {
            pasteboard.setString_forType(&text, NSPasteboardTypeString)
        };

        assert!(written);
        assert!(text_written);
        assert_eq!(
            macos_file_url_path_from(&pasteboard).as_deref(),
            Some("/Users/example/My File あ.txt")
        );
    }

    #[test]
    fn paste_text_prefers_native_file_url_path_over_clipboard_text() {
        let clipboard = TextClipboard {
            value: String::from("otty-file-icon.icns"),
        };

        assert_eq!(
            read_paste_text_from(
                &clipboard,
                Some(String::from("/Users/example/My File.txt")),
            )
            .as_deref(),
            Some("/Users/example/My File.txt")
        );
    }

    #[test]
    fn paste_text_decodes_file_url_text_fallback() {
        let clipboard = TextClipboard {
            value: String::from("file:///Users/example/My%20File.txt"),
        };

        assert_eq!(
            read_paste_text_from(&clipboard, None).as_deref(),
            Some("/Users/example/My File.txt")
        );
    }
}
