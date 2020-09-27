use crate::{utils, Error, Result};

use regex::bytes::Regex;

use std::{
    fs::File,
    io::prelude::*,
    path::{Path, PathBuf},
};

pub(crate) enum Source {
    Stdin,
    Files(Vec<PathBuf>),
}

impl Source {
    pub(crate) fn infer(file_paths: Vec<PathBuf>) -> Self {
        if file_paths.is_empty() {
            Source::Stdin
        } else {
            Source::Files(file_paths)
        }
    }
}

pub(crate) struct Replacer {
    regex: Regex,
    replace_with: Vec<u8>,
    is_literal: bool,
}

impl Replacer {
    pub(crate) fn new(
        look_for: String,
        replace_with: String,
        is_literal: bool,
        flags: Option<String>,
    ) -> Result<Self> {
        let (look_for, replace_with) = if is_literal {
            (regex::escape(&look_for), replace_with.into_bytes())
        } else {
            (
                look_for,
                utils::unescape(&replace_with)
                    .unwrap_or_else(|| replace_with)
                    .into_bytes(),
            )
        };

        let mut regex = regex::bytes::RegexBuilder::new(&look_for);
        regex.multi_line(true);

        if let Some(flags) = flags {
            flags.chars().for_each(|c| {
                #[rustfmt::skip]
                match c {
                    'c' => { regex.case_insensitive(false); },
                    'i' => { regex.case_insensitive(true); },
                    'm' => {},
                    'e' => { regex.multi_line(false); },
                    's' => {
                        if !flags.contains("m") {
                            regex.multi_line(false);
                        }
                        regex.dot_matches_new_line(true);
                    },
                    'w' => {
                        regex = regex::bytes::RegexBuilder::new(&format!(
                            "\\b{}\\b",
                            look_for
                        ));
                    },
                    _ => {},
                };
            });
        };

        Ok(Replacer {
            regex: regex.build()?,
            replace_with,
            is_literal,
        })
    }

    fn has_matches(&self, content: &[u8]) -> bool {
        self.regex.is_match(content)
    }

    fn check_not_empty(mut file: File) -> Result<()> {
        let mut buf: [u8; 1] = Default::default();
        file.read_exact(&mut buf)?;
        Ok(())
    }

    fn replace<'a>(&'a self, content: &'a [u8]) -> std::borrow::Cow<'a, [u8]> {
        if self.is_literal {
            self.regex
                .replace_all(&content, regex::bytes::NoExpand(&self.replace_with))
        } else {
            self.regex.replace_all(&content, &*self.replace_with)
        }
    }

    fn replace_file(&self, path: &Path) -> Result<()> {
        use memmap::{Mmap, MmapMut};
        use std::ops::DerefMut;

        if let Err(_) = Self::check_not_empty(File::open(path)?) {
            return Ok(());
        }

        let source = File::open(path)?;
        let meta = source.metadata()?;
        let mmap_source = unsafe { Mmap::map(&source)? };
        let replaced = self.replace(&mmap_source);

        let target = tempfile::NamedTempFile::new_in(
            path.parent()
                .ok_or_else(|| Error::InvalidPath(path.to_path_buf()))?,
        )?;
        let file = target.as_file();
        file.set_len(replaced.len() as u64)?;
        file.set_permissions(meta.permissions())?;

        if !replaced.is_empty() {
            let mut mmap_target = unsafe { MmapMut::map_mut(&file)? };
            mmap_target.deref_mut().write_all(&replaced)?;
            mmap_target.flush_async()?;
        }

        drop(mmap_source);
        drop(source);

        target.persist(path)?;
        Ok(())
    }

    pub(crate) fn run(&self, source: &Source, in_place: bool) -> Result<()> {
        match (source, in_place) {
            (Source::Stdin, _) => {
                let mut buffer = Vec::with_capacity(256);
                let stdin = std::io::stdin();
                let mut handle = stdin.lock();
                handle.read_to_end(&mut buffer)?;

                let stdout = std::io::stdout();
                let mut handle = stdout.lock();

                if self.has_matches(&buffer) {
                    handle.write_all(&self.replace(&buffer))?;
                } else {
                    handle.write_all(&buffer)?;
                }

                Ok(())
            }
            (Source::Files(paths), true) => {
                use rayon::prelude::*;

                #[allow(unused_must_use)]
                paths.par_iter().for_each(|p| {
                    self.replace_file(p)
                        .map_err(|e| eprintln!("Error processing {}: {}", p.display(), e));
                });

                Ok(())
            }
            (Source::Files(paths), false) => {
                let stdout = std::io::stdout();
                let mut handle = stdout.lock();

                paths.iter().try_for_each(|path| {
                    if let Err(_) = Self::check_not_empty(File::open(path)?) {
                        return Ok(());
                    }
                    let file = unsafe { memmap::Mmap::map(&File::open(path)?)? };
                    handle.write_all(&self.replace(&file))?;

                    Ok(())
                })
            }
        }
    }
}
