use bat::{PagingMode, PrettyPrinter};
use std::io::{self, Write};

pub fn print_themes() -> io::Result<()> {
    for theme in PrettyPrinter::new().themes() {
        writeln!(io::stdout(), "{}", theme)?;
    }
    Ok(())
}

pub fn print_colored(content: &str, theme: Option<&str>, pager: bool) -> bat::error::Result<()> {
    let mut pretty_printer = PrettyPrinter::new();
    pretty_printer
        .input_from_bytes(content.as_bytes())
        .language("rust")
        .tab_width(Some(4))
        .true_color(false)
        .header(false)
        .line_numbers(false)
        .grid(false);
    if let Some(theme) = theme {
        pretty_printer.theme(theme);
    }
    if pager {
        pretty_printer.paging_mode(PagingMode::QuitIfOneScreen);
    }
    pretty_printer.print()?;
    Ok(())
}
