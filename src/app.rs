use structopt::{clap::AppSettings, StructOpt};

#[derive(Debug, StructOpt)]
#[structopt(
    setting(AppSettings::ColoredHelp),
    setting(AppSettings::NextLineHelp),
    setting(AppSettings::UnifiedHelpMessage)
)]
pub(crate) struct Options {
    #[structopt(short = "p", long = "preview")]
    /// Ouput result into stdout and do not modify files.
    pub preview: bool,

    #[structopt(short = "s", long = "string-mode")]
    /// Thread expressions as non-regex strings.
    pub literal_mode: bool,

    #[structopt(short = "f", long = "flags", verbatim_doc_comment)]
    #[rustfmt::skip]
    /** Regex flags. Maybe combined (like `-f mc`).

c - case-sensitive
e - disable multi-line matching
i - case-insensitive
m - multi-line matching
s - make `.` match newlines
w - match full words only
{n}
    */
    pub flags: Option<String>,

    /// The regexp or string (if -s) to search for.
    pub find: String,

    /// What to replace each match with. Unless in string mode, you may
    /// use captured values like $1, $2, etc.
    pub replace_with: String,

    /// The path to file(s). This is optional - sd can also read from STDIN.
    /// Note: sd modifies files in-place by default. See documentation for
    /// examples.
    pub files: Vec<std::path::PathBuf>,
}
