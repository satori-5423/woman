use woman::locale;

#[test]
fn test_is_english_locale() {
    // English locales should be detected
    assert!(locale::is_english_locale("en_US.UTF-8"));
    assert!(locale::is_english_locale("en_GB.UTF-8"));
    assert!(locale::is_english_locale("en_AU"));
    assert!(locale::is_english_locale("C"));
    assert!(locale::is_english_locale("POSIX"));
    assert!(locale::is_english_locale("c"));
    assert!(locale::is_english_locale("posix"));
    assert!(locale::is_english_locale("EN_us"));

    // Non-English locales should not be detected
    assert!(!locale::is_english_locale("zh_CN.UTF-8"));
    assert!(!locale::is_english_locale("ja_JP.UTF-8"));
    assert!(!locale::is_english_locale("fr_FR.UTF-8"));
    assert!(!locale::is_english_locale("de_DE"));
    assert!(!locale::is_english_locale(""));
}

#[test]
fn test_get_target_locale() {
    // This test can only verify the fallback path in a controlled environment.
    // We test the function's fallback logic by temporarily clearing env vars.
    let saved_lc_all = std::env::var("LC_ALL").ok();
    let saved_lc_messages = std::env::var("LC_MESSAGES").ok();
    let saved_lang = std::env::var("LANG").ok();

    unsafe {
        std::env::remove_var("LC_ALL");
        std::env::remove_var("LC_MESSAGES");
        std::env::remove_var("LANG");
    }

    let result = locale::get_target_locale();
    assert_eq!(result, "C");

    // Restore
    if let Some(v) = saved_lc_all {
        unsafe {
            std::env::set_var("LC_ALL", v);
        }
    }
    if let Some(v) = saved_lc_messages {
        unsafe {
            std::env::set_var("LC_MESSAGES", v);
        }
    }
    if let Some(v) = saved_lang {
        unsafe {
            std::env::set_var("LANG", v);
        }
    }
}

#[test]
fn test_should_bypass_translation() {
    // Help and version flags should bypass
    assert!(locale::should_bypass_translation(&["-h".to_string()]));
    assert!(locale::should_bypass_translation(&["--help".to_string()]));
    assert!(locale::should_bypass_translation(&["-V".to_string()]));
    assert!(locale::should_bypass_translation(
        &["--version".to_string()]
    ));

    // Search-related flags should bypass
    assert!(locale::should_bypass_translation(&["-k".to_string()]));
    assert!(locale::should_bypass_translation(
        &["--apropos".to_string()]
    ));
    assert!(locale::should_bypass_translation(&["-f".to_string()]));
    assert!(locale::should_bypass_translation(&["--whatis".to_string()]));
    assert!(locale::should_bypass_translation(&["-K".to_string()]));
    assert!(locale::should_bypass_translation(&[
        "--global-apropos".to_string()
    ]));

    // Normal man page requests should not bypass
    assert!(!locale::should_bypass_translation(&["ls".to_string()]));
    assert!(!locale::should_bypass_translation(&[
        "1".to_string(),
        "ls".to_string()
    ]));
    assert!(!locale::should_bypass_translation(&[
        "-w".to_string(),
        "ls".to_string()
    ]));
    assert!(!locale::should_bypass_translation(&[]));

    // Local file flags should bypass
    assert!(locale::should_bypass_translation(&["-l".to_string()]));
    assert!(locale::should_bypass_translation(&[
        "--local-file".to_string()
    ]));

    // Bypass flags mixed with other args should still bypass
    assert!(locale::should_bypass_translation(&[
        "-k".to_string(),
        "ls".to_string()
    ]));
    assert!(locale::should_bypass_translation(&[
        "ls".to_string(),
        "--help".to_string()
    ]));

    // Combined short options containing a bypass flag should bypass
    assert!(locale::should_bypass_translation(&["-kh".to_string()]));
    assert!(locale::should_bypass_translation(&[
        "-lk".to_string(),
        "ls".to_string()
    ]));

    // Combined flags WITHOUT a bypass flag must NOT bypass
    assert!(!locale::should_bypass_translation(&[
        "-aw".to_string(),
        "ls".to_string()
    ]));
    assert!(!locale::should_bypass_translation(&[
        "-P".to_string(),
        "cat".to_string(),
        "ls".to_string()
    ]));
    assert!(!locale::should_bypass_translation(&["-t".to_string()]));
}
