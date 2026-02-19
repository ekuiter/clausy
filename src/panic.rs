use std::io::IsTerminal;

fn format_panic_message(message: &str) -> (&str, Option<&str>) {
    for marker in [": Os {", ": Custom {"] {
        if let Some(idx) = message.find(marker) {
            return (&message[..idx], Some(&message[idx + 2..]));
        }
    }
    (message, None)
}

#[derive(Clone, Copy)]
struct Theme {
    reset: &'static str,
    header: &'static str,
    label: &'static str,
    message: &'static str,
    exception: &'static str,
    location: &'static str,
}

impl Theme {
    fn plain() -> Self {
        Self {
            reset: "",
            header: "",
            label: "",
            message: "",
            exception: "",
            location: "",
        }
    }

    fn colored() -> Self {
        Self {
            reset: "\x1b[0m",
            header: "\x1b[1;31m",
            label: "\x1b[1;36m",
            message: "\x1b[97m",
            exception: "\x1b[93m",
            location: "\x1b[90m",
        }
    }
}

fn use_color() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stderr().is_terminal()
}

fn backtrace_requested() -> bool {
    matches!(
        std::env::var("RUST_BACKTRACE").ok().as_deref(),
        Some("1" | "full")
    )
}

pub(crate) fn install_panic_hook() {
    let theme = if use_color() {
        Theme::colored()
    } else {
        Theme::plain()
    };
    std::panic::set_hook(Box::new(move |info| {
        let payload = info
            .payload()
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| info.payload().downcast_ref::<&str>().copied())
            .unwrap_or("panic without message");
        let (message, exception) = format_panic_message(payload);

        eprintln!("{}clausy error{}", theme.header, theme.reset);
        eprintln!(
            "{}Message{}   {}{}{}",
            theme.label, theme.reset, theme.message, message, theme.reset
        );
        if let Some(exception) = exception {
            eprintln!(
                "{}Exception{} {}{}{}",
                theme.label, theme.reset, theme.exception, exception, theme.reset
            );
        } else {
            eprintln!(
                "{}Exception{} {}panic{}",
                theme.label, theme.reset, theme.exception, theme.reset
            );
        }
        if let Some(location) = info.location() {
            eprintln!(
                "{}Location{}  {}{}:{}:{}{}",
                theme.label,
                theme.reset,
                theme.location,
                location.file(),
                location.line(),
                location.column(),
                theme.reset
            );
        } else {
            eprintln!(
                "{}Location{}  {}unknown{}",
                theme.label, theme.reset, theme.location, theme.reset
            );
        }
        if backtrace_requested() {
            let backtrace = std::backtrace::Backtrace::force_capture();
            eprintln!(
                "{}Backtrace{}\n{}{}{}",
                theme.label, theme.reset, theme.location, backtrace, theme.reset
            );
        } else {
            eprintln!(
                "{}note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace{}",
                theme.location, theme.reset
            );
        }
    }));
}
