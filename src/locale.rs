/// Determine the target locale from environment variables.
/// Checks LC_ALL, then LC_MESSAGES, then LANG, falling back to zh_CN.UTF-8.
pub fn get_target_locale() -> String {
    std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "zh_CN.UTF-8".to_string())
}

/// Check whether the given locale string represents an English-speaking locale.
/// Returns true for C, POSIX, and any locale starting with "en" (case-insensitive).
pub fn is_english_locale(locale: &str) -> bool {
    let l = locale.to_lowercase();
    l == "c" || l == "posix" || l.starts_with("en")
}

/// Determine whether the given man arguments indicate a non-translation operation
/// (help, version, keyword search, etc.) that should bypass the translation process.
pub fn should_bypass_translation(args: &[String]) -> bool {
    for arg in args {
        if arg == "-h"
            || arg == "--help"
            || arg == "-V"
            || arg == "--version"
            || arg == "-k"
            || arg == "--apropos"
            || arg == "-f"
            || arg == "--whatis"
            || arg == "-K"
            || arg == "--global-apropos"
            || arg == "-l"
            || arg == "--local-file"
        {
            return true;
        }
    }
    false
}
