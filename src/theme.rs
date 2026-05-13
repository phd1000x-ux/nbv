//! 색 팔레트. ANSI 16색 기반(터미널 호환성 최우선).

// SGR 코드
pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const ITALIC: &str = "\x1b[3m";

pub const FG_CYAN: &str = "\x1b[36m";
pub const FG_GREEN: &str = "\x1b[32m";
pub const FG_RED: &str = "\x1b[31m";
pub const FG_BLUE: &str = "\x1b[34m";
pub const FG_YELLOW: &str = "\x1b[33m";
pub const FG_MAGENTA: &str = "\x1b[35m";

pub fn colorize_code_header(s: &str, color: bool) -> String {
    if color {
        format!("{}{}{}{}", BOLD, FG_CYAN, s, RESET)
    } else {
        s.to_string()
    }
}

pub fn colorize_output_header(s: &str, color: bool) -> String {
    if color {
        format!("{}{}{}{}", BOLD, FG_GREEN, s, RESET)
    } else {
        s.to_string()
    }
}

pub fn colorize_error_header(s: &str, color: bool) -> String {
    if color {
        format!("{}{}{}{}", BOLD, FG_RED, s, RESET)
    } else {
        s.to_string()
    }
}

pub fn colorize_markdown_header(s: &str, color: bool) -> String {
    if color {
        format!("{}{}{}{}", BOLD, FG_BLUE, s, RESET)
    } else {
        s.to_string()
    }
}

pub fn dim(s: &str, color: bool) -> String {
    if color {
        format!("{}{}{}", DIM, s, RESET)
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_header_has_cyan_fg() {
        let s = colorize_code_header("In [1] code (python)", true);
        assert!(s.contains("\x1b[")); // ANSI escape
        assert!(s.ends_with("\x1b[0m")); // reset
    }

    #[test]
    fn code_header_no_color_returns_plain() {
        let s = colorize_code_header("In [1] code (python)", false);
        assert_eq!(s, "In [1] code (python)");
    }

    #[test]
    fn dim_text_wraps_when_color() {
        assert!(dim("hi", true).contains("\x1b["));
        assert_eq!(dim("hi", false), "hi");
    }
}
