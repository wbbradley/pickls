use crate::prelude::*;
use regex::Regex;

fn parse_multi_regex_query(query: &str) -> Result<Vec<Regex>> {
    let mut regexes: Vec<Regex> = Default::default();
    for regex in query.split_whitespace().map(Regex::new) {
        let regex = regex?;
        regexes.push(regex);
    }
    Ok(regexes)
}
fn construct_ctags_command(folders: &Vec<PathBuf>, excludes: &Vec<String>) -> Result<Command> {
    let mut cmd = Command::new("ctags");
    cmd.kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    cmd.arg("--options=NONE")
        .arg("--fields=+K")
        .arg("--extras=+q")
        .arg("--excmd=number")
        .arg("--recurse")
        .arg("--sort=no")
        .arg("-f")
        .arg("-");

    for exclude in excludes {
        cmd.arg(format!(
            "--exclude={}",
            shlex::try_quote(exclude).expect("failed to quote an exclude")
        ));
    }
    for folder in folders {
        // Force ctags to use the canonical path.
        cmd.arg(folder.canonicalize()?);
    }
    Ok(cmd)
}

pub(crate) async fn find_symbols(
    query: &str,
    folders: &Vec<PathBuf>,
    excludes: &Vec<String>,
    max_symbols: usize,
) -> Result<Vec<SymbolInformation>> {
    parse_ctags_output(
        parse_multi_regex_query(query)?,
        construct_ctags_command(folders, excludes)?.spawn()?,
        max_symbols,
    )
    .await
}

pub(crate) async fn parse_ctags_output(
    regexes: Vec<Regex>,
    mut proc: tokio::process::Child,
    max_symbols: usize,
) -> Result<Vec<SymbolInformation>> {
    let mut symbols = Vec::with_capacity(max_symbols);
    let stdout = BufReader::new(
        proc.stdout
            .take()
            .ok_or_else(|| Error::new("Failed to capture child process stdout"))?,
    );
    let mut reader = stdout.lines();
    while let Some(line) = reader.next_line().await? {
        log::info!("line: {line}");
        if symbols.len() >= max_symbols {
            break;
        }
        // log::info!("parsing ctags line: {line}");
        let Some((tag, path, line_number, kind)) = parse_ctags_line(&line) else {
            log::info!("failed to parse ctags line [line='{line}']");
            continue;
        };
        if !regexes.iter().all(|re| re.is_match(tag)) {
            log::trace!("skipping line due to regexes [line='{line}', regexes={regexes:?}]");
            continue;
        }
        if let Ok(path) = PathBuf::from(path).canonicalize() {
            if let Ok(uri) = Url::from_file_path(path) {
                #[allow(deprecated)]
                let symbol = SymbolInformation {
                    name: tag.to_string(),
                    kind,
                    location: Location {
                        uri,
                        range: Range {
                            start: Position {
                                line: line_number - 1,
                                character: 0,
                            },
                            end: Position {
                                line: line_number,
                                character: 0,
                            },
                        },
                    },
                    tags: None,
                    deprecated: None,
                    container_name: None,
                };
                symbols.push(symbol);
            }
        }
    }
    dbg!(&symbols);
    Ok(symbols)
}

fn parse_ctags_line(line: &str) -> Option<(&str, &str, u32, SymbolKind)> {
    let mut term_iter = line.split('\t');
    let tag = term_iter.next()?;
    let path = term_iter.next()?;
    let line_number: u32 = term_iter.next()?.split_once(";")?.0.parse().ok()?;
    let kind = convert_kind(term_iter.next()?);
    Some((tag, path, line_number, kind))
}

fn convert_kind(kind: &str) -> SymbolKind {
    let term = if let Some((fst, _)) = kind.split_once(':') {
        fst
    } else {
        kind
    };

    match term {
        "function" => SymbolKind::FUNCTION,
        "class" => SymbolKind::CLASS,
        "variable" => SymbolKind::VARIABLE,
        "method" => SymbolKind::METHOD,
        "module" => SymbolKind::MODULE,
        // Add more kind mappings as necessary
        _ => SymbolKind::VARIABLE,
    }
}
