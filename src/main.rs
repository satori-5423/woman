use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};

use woman::config;
use woman::gzip;
use woman::locale;
use woman::models;
use woman::translator;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && (args[1] == "model" || args[1] == "--model") {
        handle_model_subcommand(&args);
        return;
    }

    let locale = locale::get_target_locale();

    if args.len() == 1 {
        let err = std::process::Command::new("man").exec();
        eprintln!("Failed to execute man: {}", err);
        std::process::exit(1);
    }

    if locale::is_english_locale(&locale) || locale::should_bypass_translation(&args[1..]) {
        let err = std::process::Command::new("man").args(&args[1..]).exec();
        eprintln!("Failed to execute man: {}", err);
        std::process::exit(1);
    }

    let output = {
        let mut cmd = std::process::Command::new("man");
        cmd.arg("-w").args(&args[1..]);

        // When running under fish, prepend fish's man directory to MANPATH
        // so that fish-specific man pages (e.g., builtin commands) take priority.
        // This replicates what fish's internal `man` wrapper function does.
        //
        // In man-db, a trailing colon in MANPATH inserts the system default
        // search path at that point, so "<fish>:" means "fish first, then default".
        if let Some(fish_man_dir) = get_fish_man_dir() {
            let existing = std::env::var("MANPATH").unwrap_or_default();
            let new_manpath = if existing.is_empty() {
                format!("{}:", fish_man_dir.display())
            } else {
                format!("{}:{}", fish_man_dir.display(), existing)
            };
            cmd.env("MANPATH", new_manpath);
        }

        // Use LC_ALL=C to always locate the original English man page for translation.
        cmd.env("LC_ALL", "C");

        cmd.output()
    };

    let output = match output {
        Ok(out) => out,
        Err(e) => {
            eprintln!("Error executing man -w: {}", e);
            std::process::exit(1);
        }
    };

    if !output.status.success() {
        let _ = std::io::stderr().write_all(&output.stderr);
        let _ = std::io::stdout().write_all(&output.stdout);
        std::process::exit(output.status.code().unwrap_or(1));
    }

    let paths_str = String::from_utf8_lossy(&output.stdout);
    let original_paths: Vec<&str> = paths_str
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if original_paths.is_empty() {
        std::process::exit(0);
    }

    let cfg = config::Config::load();
    if cfg.active_model.is_empty() {
        eprintln!("Error: Active model is not configured.");
        eprintln!("Please set it using: woman model set-model <model_name>");
        std::process::exit(1);
    }
    if cfg.api_key.is_empty() {
        eprintln!("Error: API key is not configured.");
        eprintln!("Please set it using: woman model set-key <api_key>");
        std::process::exit(1);
    }
    let api_url = models::resolve_api_url(&cfg.api_url, &cfg.active_model);

    let mut translated_paths = Vec::new();
    for path_str in &original_paths {
        let original_path = Path::new(path_str);
        let cached_path = match get_translated_path(original_path, &locale) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        };

        if !cached_path.exists() {
            eprintln!(
                "[woman] Translating {} to {} using model {}...",
                original_path.display(),
                locale,
                cfg.active_model
            );

            let raw_content = gzip::read_gzip_file(original_path).unwrap_or_else(|e| {
                eprintln!("Error reading source file: {}", e);
                std::process::exit(1);
            });

            let translated_content =
                translator::translate_man_page(&raw_content, &locale, &cfg, &api_url)
                    .unwrap_or_else(|e| {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    });

            if let Err(e) = gzip::write_gzip_file(&cached_path, &translated_content) {
                eprintln!("Error writing cached translation: {}", e);
                std::process::exit(1);
            }
        }

        translated_paths.push(cached_path);
    }

    // Ensure all cached translations have `.ad l` to prevent groff
    // from adding excessive spaces during text justification.
    for p in &translated_paths {
        if let Err(e) = translator::ensure_cached_file_has_no_adjust(p) {
            eprintln!("Warning: failed to fix up {}: {}", p.display(), e);
        }
    }

    let has_w = args[1..]
        .iter()
        .any(|arg| arg == "-w" || arg == "--path" || arg == "--where");

    if has_w {
        for p in translated_paths {
            println!("{}", p.to_string_lossy());
        }
        std::process::exit(0);
    }

    // Build arguments for the final man invocation.
    // Pass through flags (like -P, -T) but exclude -w/--path/--where.
    // Section numbers and page names are dropped because we pass the
    // translated file paths directly, which encode the section.
    let mut man_args: Vec<String> = args[1..]
        .iter()
        .filter(|arg| arg.starts_with('-') && *arg != "-w" && *arg != "--path" && *arg != "--where")
        .cloned()
        .collect();

    for p in translated_paths {
        man_args.push(p.to_string_lossy().into_owned());
    }

    let err = std::process::Command::new("man").args(&man_args).exec();
    eprintln!("Failed to execute man: {}", err);
    std::process::exit(1);
}

fn handle_model_subcommand(args: &[String]) {
    let subcommand = args.get(2).map(|s| s.as_str()).unwrap_or("help");
    match subcommand {
        "list" => cmd_list_models(),
        "show" => cmd_show_config(),
        "set-model" => cmd_set_model(args),
        "set-key" => cmd_set_key(args),
        "set-url" => cmd_set_url(args),
        _ => print_model_help(),
    }
}

fn cmd_list_models() {
    let cfg = config::Config::load();

    if cfg.api_key.is_empty() {
        eprintln!("Error: API key is not configured.");
        eprintln!("Please set it first: woman model set-key <api_key>");
        std::process::exit(1);
    }

    let api_url = models::resolve_api_url(&cfg.api_url, &cfg.active_model);
    let provider_name = models::infer_provider_from_model(&cfg.active_model);

    eprintln!("[woman] Fetching available models from {}...", api_url);

    match models::get_provider(provider_name) {
        Some(provider) => {
            println!("Available Models — {}:", provider.name());
            match provider.fetch_models(&cfg.api_key, &api_url) {
                Ok(model_list) => {
                    if model_list.is_empty() {
                        println!("  (no models returned)");
                    } else {
                        for m in &model_list {
                            println!("  * {}", m.id);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching models: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            eprintln!(
                "Error: No model provider found for model '{}'.",
                cfg.active_model
            );
            eprintln!("Supported providers: {:?}", models::list_providers());
            std::process::exit(1);
        }
    }
}

fn cmd_show_config() {
    let cfg = config::Config::load();
    let api_url = models::resolve_api_url(&cfg.api_url, &cfg.active_model);

    println!(
        "Active Model: {}",
        if cfg.active_model.is_empty() {
            "<not set>"
        } else {
            &cfg.active_model
        }
    );
    println!("API URL:      {}", api_url);
    if !cfg.api_url.is_empty() {
        println!("  (custom override)");
    }

    let masked_key = mask_api_key(&cfg.api_key);
    println!("API Key:      {}", masked_key);
}

fn cmd_set_model(args: &[String]) {
    if args.len() < 4 {
        eprintln!("Error: Please specify the model name.");
        eprintln!("Usage: woman model set-model <model_name>");
        std::process::exit(1);
    }
    let mut cfg = config::Config::load();
    cfg.active_model = args[3].clone();
    cfg.save().unwrap_or_else(|e| {
        eprintln!("Error saving config: {}", e);
        std::process::exit(1);
    });
    println!("Model updated to: {}", cfg.active_model);
}

fn cmd_set_key(args: &[String]) {
    if args.len() < 4 {
        eprintln!("Error: Please specify the API key.");
        eprintln!("Usage: woman model set-key <api_key>");
        std::process::exit(1);
    }
    let mut cfg = config::Config::load();
    cfg.api_key = args[3].clone();
    cfg.save().unwrap_or_else(|e| {
        eprintln!("Error saving config: {}", e);
        std::process::exit(1);
    });
    println!("API key updated successfully.");
}

fn cmd_set_url(args: &[String]) {
    if args.len() < 4 {
        eprintln!("Error: Please specify the API base URL.");
        eprintln!("Usage: woman model set-url <api_url>");
        std::process::exit(1);
    }
    let mut cfg = config::Config::load();
    cfg.api_url = args[3].clone();
    cfg.save().unwrap_or_else(|e| {
        eprintln!("Error saving config: {}", e);
        std::process::exit(1);
    });
    println!("API URL updated to: {}", cfg.api_url);
}

fn print_model_help() {
    println!("woman model management commands:");
    println!("  woman model list              List available models");
    println!("  woman model show              Show current configuration");
    println!("  woman model set-model <name>  Set the active model name");
    println!("  woman model set-key <key>     Set the API key for the active model");
    println!("  woman model set-url <url>     Set the custom API base URL");
}

fn mask_api_key(key: &str) -> String {
    if key.is_empty() {
        "<not set>".to_string()
    } else if key.len() > 8 {
        format!("{}••••••••{}", &key[..4], &key[key.len() - 4..])
    } else {
        "••••••••".to_string()
    }
}

fn get_translated_path(original_path: &Path, locale: &str) -> Result<PathBuf, String> {
    let woman_dir = config::get_woman_dir()?;
    let translations_dir = woman_dir.join("translations").join(locale);

    let mut components = original_path.components();
    if original_path.is_absolute() {
        components.next();
    }

    Ok(translations_dir.join(components.as_path()))
}

/// Returns the fish shell's man directory if it exists.
/// Fish ships its own man pages (for builtins) under
/// `$__fish_man_dir` which defaults to e.g. `/usr/share/fish/man`.
/// When running under fish, prepend this directory to MANPATH
/// so that `man -w` finds fish-specific pages before the
/// system-wide POSIX variants.
fn get_fish_man_dir() -> Option<PathBuf> {
    // Check the standard install locations for fish.
    let candidates = ["/usr/share/fish/man", "/usr/local/share/fish/man"];
    for dir in &candidates {
        let path = Path::new(dir);
        if path.is_dir() {
            return Some(path.to_path_buf());
        }
    }
    None
}
