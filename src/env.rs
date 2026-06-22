#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageBackend {
    Kitty,
    ITerm2,
    Placeholder,
}

#[derive(Debug, Clone)]
pub struct RenderCtx {
    pub is_tty: bool,
    pub use_color: bool,
    pub width: usize,
    pub image_backend: ImageBackend,
    pub code_theme: String,
}

pub trait EnvProbe {
    fn is_tty(&self) -> bool;
    fn no_color(&self) -> bool;
    fn term_program(&self) -> Option<String>;
    fn term(&self) -> Option<String>;
    fn columns(&self) -> Option<usize>;
}

pub struct SystemEnv;

impl EnvProbe for SystemEnv {
    fn is_tty(&self) -> bool {
        use std::io::IsTerminal;
        std::io::stdout().is_terminal()
    }
    fn no_color(&self) -> bool {
        std::env::var_os("NO_COLOR").is_some()
    }
    fn term_program(&self) -> Option<String> {
        std::env::var("TERM_PROGRAM").ok()
    }
    fn term(&self) -> Option<String> {
        std::env::var("TERM").ok()
    }
    fn columns(&self) -> Option<usize> {
        crossterm::terminal::size().ok().map(|(w, _)| w as usize)
    }
}

#[cfg(test)]
pub struct TestEnv {
    pub is_tty: bool,
    pub no_color: bool,
    pub term_program: Option<String>,
    pub term: Option<String>,
    pub columns: Option<usize>,
}

#[cfg(test)]
impl EnvProbe for TestEnv {
    fn is_tty(&self) -> bool {
        self.is_tty
    }
    fn no_color(&self) -> bool {
        self.no_color
    }
    fn term_program(&self) -> Option<String> {
        self.term_program.clone()
    }
    fn term(&self) -> Option<String> {
        self.term.clone()
    }
    fn columns(&self) -> Option<usize> {
        self.columns
    }
}

pub fn detect(
    args_no_color: bool,
    args_no_images: bool,
    args_theme: Option<String>,
    args_width: Option<usize>,
) -> RenderCtx {
    detect_with(
        &SystemEnv,
        args_no_color,
        args_no_images,
        args_theme,
        args_width,
    )
}

/// Pick the image backend from terminal identity. Forced to `Placeholder` when
/// images are disabled or output is not a TTY.
fn detect_image_backend(env: &impl EnvProbe, no_images: bool, is_tty: bool) -> ImageBackend {
    if no_images || !is_tty {
        ImageBackend::Placeholder
    } else if env.term_program().as_deref() == Some("ghostty")
        || env.term().as_deref() == Some("xterm-kitty")
    {
        ImageBackend::Kitty
    } else if env.term_program().as_deref() == Some("iTerm.app") {
        ImageBackend::ITerm2
    } else {
        ImageBackend::Placeholder
    }
}

pub fn detect_with(
    env: &impl EnvProbe,
    args_no_color: bool,
    args_no_images: bool,
    args_theme: Option<String>,
    args_width: Option<usize>,
) -> RenderCtx {
    let is_tty = env.is_tty();
    let use_color = is_tty && !args_no_color && !env.no_color();
    let width = args_width.or_else(|| env.columns()).unwrap_or(80);

    let image_backend = detect_image_backend(env, args_no_images, is_tty);

    let code_theme = args_theme.unwrap_or_else(|| "base16-ocean.dark".to_string());

    RenderCtx {
        is_tty,
        use_color,
        width,
        image_backend,
        code_theme,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghostty_tty_picks_kitty() {
        let env = TestEnv {
            is_tty: true,
            no_color: false,
            term_program: Some("ghostty".into()),
            term: Some("xterm-ghostty".into()),
            columns: Some(120),
        };
        let ctx = detect_with(
            &env, /* args.no_color */ false, /* args.no_images */ false, None, None,
        );
        assert_eq!(ctx.image_backend, ImageBackend::Kitty);
        assert!(ctx.use_color);
        assert!(ctx.is_tty);
        assert_eq!(ctx.width, 120);
    }

    #[test]
    fn iterm_tty_picks_iterm2() {
        let env = TestEnv {
            is_tty: true,
            no_color: false,
            term_program: Some("iTerm.app".into()),
            term: None,
            columns: None,
        };
        let ctx = detect_with(&env, false, false, None, None);
        assert_eq!(ctx.image_backend, ImageBackend::ITerm2);
        assert_eq!(ctx.width, 80); // default
    }

    #[test]
    fn kitty_term_var_picks_kitty() {
        let env = TestEnv {
            is_tty: true,
            no_color: false,
            term_program: None,
            term: Some("xterm-kitty".into()),
            columns: None,
        };
        let ctx = detect_with(&env, false, false, None, None);
        assert_eq!(ctx.image_backend, ImageBackend::Kitty);
    }

    #[test]
    fn other_terminal_picks_placeholder() {
        let env = TestEnv {
            is_tty: true,
            no_color: false,
            term_program: Some("Apple_Terminal".into()),
            term: Some("xterm-256color".into()),
            columns: None,
        };
        let ctx = detect_with(&env, false, false, None, None);
        assert_eq!(ctx.image_backend, ImageBackend::Placeholder);
    }

    #[test]
    fn non_tty_forces_placeholder_and_no_color() {
        let env = TestEnv {
            is_tty: false,
            no_color: false,
            term_program: Some("ghostty".into()),
            term: None,
            columns: None,
        };
        let ctx = detect_with(&env, false, false, None, None);
        assert_eq!(ctx.image_backend, ImageBackend::Placeholder);
        assert!(!ctx.use_color);
    }

    #[test]
    fn no_color_env_var_disables_color() {
        let env = TestEnv {
            is_tty: true,
            no_color: true,
            term_program: Some("ghostty".into()),
            term: None,
            columns: None,
        };
        let ctx = detect_with(&env, false, false, None, None);
        assert!(!ctx.use_color);
    }

    #[test]
    fn no_color_flag_overrides() {
        let env = TestEnv {
            is_tty: true,
            no_color: false,
            term_program: None,
            term: None,
            columns: None,
        };
        let ctx = detect_with(&env, /* no_color */ true, false, None, None);
        assert!(!ctx.use_color);
    }

    #[test]
    fn no_images_flag_forces_placeholder() {
        let env = TestEnv {
            is_tty: true,
            no_color: false,
            term_program: Some("ghostty".into()),
            term: None,
            columns: None,
        };
        let ctx = detect_with(&env, false, /* no_images */ true, None, None);
        assert_eq!(ctx.image_backend, ImageBackend::Placeholder);
    }

    #[test]
    fn theme_arg_overrides_default() {
        let env = TestEnv {
            is_tty: true,
            no_color: false,
            term_program: Some("ghostty".into()),
            term: None,
            columns: Some(120),
        };
        let ctx = detect_with(&env, false, false, Some("InspiredGitHub".into()), None);
        assert_eq!(ctx.code_theme, "InspiredGitHub");
    }

    #[test]
    fn default_theme_when_arg_none() {
        let env = TestEnv {
            is_tty: true,
            no_color: false,
            term_program: Some("ghostty".into()),
            term: None,
            columns: Some(120),
        };
        let ctx = detect_with(&env, false, false, None, None);
        assert_eq!(ctx.code_theme, "base16-ocean.dark");
    }

    #[test]
    fn width_arg_overrides_env_columns() {
        let env = TestEnv {
            is_tty: true,
            no_color: false,
            term_program: Some("ghostty".into()),
            term: None,
            columns: Some(80),
        };
        let ctx = detect_with(&env, false, false, None, Some(120));
        assert_eq!(ctx.width, 120);
    }

    #[test]
    fn width_falls_back_to_env_then_default() {
        let env = TestEnv {
            is_tty: true,
            no_color: false,
            term_program: None,
            term: None,
            columns: None,
        };
        let ctx = detect_with(&env, false, false, None, None);
        assert_eq!(ctx.width, 80);
    }
}
