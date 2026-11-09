mod app;

use cosmic_applet_shortcut_cheatsheet::shortcuts::Cheatsheet;

fn main() -> cosmic::iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let arguments: Vec<_> = std::env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("--help" | "-h") if arguments.len() == 1 => {
            println!(
                "COSMIC Shortcut Cheatsheet\n\n\
                 Usage: cosmic-applet-shortcut-cheatsheet [OPTION]\n\n\
                 (no option)         Run as a COSMIC panel applet\n\
                 --preview           Preview the cheatsheet in a regular window\n\
                 --print-shortcuts   Print the effective COSMIC shortcuts\n\
                 --version           Print version\n\
                 --help              Show this help"
            );
            Ok(())
        }
        Some("--version") if arguments.len() == 1 => {
            println!(
                "cosmic-applet-shortcut-cheatsheet {}",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
        Some("--print-shortcuts") if arguments.len() == 1 => {
            match Cheatsheet::load() {
                Ok(sheet) => sheet.print(),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            }
            Ok(())
        }
        Some("--preview") if arguments.len() == 1 => cosmic::app::run::<app::Applet>(
            cosmic::app::Settings::default()
                .size(cosmic::iced::Size::new(app::WIDTH, app::MAX_HEIGHT))
                .transparent(false),
            true,
        ),
        None => cosmic::applet::run::<app::Applet>(false),
        _ => {
            eprintln!("Unknown option. Run with --help for usage.");
            std::process::exit(2);
        }
    }
}
