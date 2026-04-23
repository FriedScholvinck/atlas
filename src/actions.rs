use crate::model::{Kind, SoftwareItem, Source};
use crate::probe::Available;
use anyhow::{bail, Result};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ShellCmd {
    pub exe: PathBuf,
    pub args: Vec<String>,
    pub display: String,
}

impl ShellCmd {
    pub fn new(exe: PathBuf, args: Vec<String>, display: impl Into<String>) -> Self {
        Self {
            exe,
            args,
            display: display.into(),
        }
    }

    pub fn run_inherited(&self) -> Result<i32> {
        let status = Command::new(&self.exe).args(&self.args).status()?;
        Ok(status.code().unwrap_or(-1))
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    Delete { name: String, cmd: ShellCmd },
    Update { name: String, cmd: ShellCmd },
    UpdateAll { chain: Vec<ShellCmd> },
}

impl Action {
    pub fn title(&self) -> String {
        match self {
            Action::Delete { name, .. } => format!("Delete {}", name),
            Action::Update { name, .. } => format!("Update {}", name),
            Action::UpdateAll { .. } => "Update everything".into(),
        }
    }

    pub fn display_cmds(&self) -> Vec<String> {
        match self {
            Action::Delete { cmd, .. } | Action::Update { cmd, .. } => vec![cmd.display.clone()],
            Action::UpdateAll { chain } => chain.iter().map(|c| c.display.clone()).collect(),
        }
    }

    pub fn is_destructive(&self) -> bool {
        matches!(self, Action::Delete { .. })
    }

    pub fn run(&self) -> Result<()> {
        match self {
            Action::Delete { cmd, .. } | Action::Update { cmd, .. } => {
                let code = cmd.run_inherited()?;
                if code != 0 {
                    bail!("command exited with {code}");
                }
            }
            Action::UpdateAll { chain } => {
                for cmd in chain {
                    println!("\n→ {}", cmd.display);
                    io::stdout().flush().ok();
                    let code = cmd.run_inherited()?;
                    if code != 0 {
                        bail!("{} exited with {code}", cmd.display);
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn delete_for(item: &SoftwareItem, av: &Available) -> Option<Action> {
    match item.source {
        Source::Brew => av.brew.as_ref().map(|exe| Action::Delete {
            name: item.name.clone(),
            cmd: brew_like(exe, "uninstall", &item.name, item.kind, "brew"),
        }),
        Source::Zerobrew => av.zb.as_ref().map(|exe| Action::Delete {
            name: item.name.clone(),
            cmd: zb_cmd(exe, "uninstall", &item.name),
        }),
        Source::Manual => item.install_path.as_ref().map(|p| Action::Delete {
            name: item.name.clone(),
            cmd: trash_cmd(p),
        }),
        _ => None, // AppStore needs sudo; lang managers deferred.
    }
}

pub fn update_for(item: &SoftwareItem, av: &Available) -> Option<Action> {
    match item.source {
        Source::Brew => av.brew.as_ref().map(|exe| Action::Update {
            name: item.name.clone(),
            cmd: brew_like(exe, "upgrade", &item.name, item.kind, "brew"),
        }),
        Source::Zerobrew => av.zb.as_ref().map(|exe| Action::Update {
            name: item.name.clone(),
            cmd: zb_cmd(exe, "upgrade", &item.name),
        }),
        _ => None,
    }
}

pub fn update_all(av: &Available) -> Option<Action> {
    let mut chain = vec![];
    if let Some(exe) = &av.zb {
        chain.push(ShellCmd::new(
            exe.clone(),
            vec!["upgrade".into()],
            "zb upgrade",
        ));
    }
    if let Some(exe) = &av.brew {
        chain.push(ShellCmd::new(
            exe.clone(),
            vec!["upgrade".into()],
            "brew upgrade",
        ));
    }
    if let Some(exe) = &av.mas {
        chain.push(ShellCmd::new(
            exe.clone(),
            vec!["upgrade".into()],
            "mas upgrade",
        ));
    }
    if chain.is_empty() {
        None
    } else {
        Some(Action::UpdateAll { chain })
    }
}

fn brew_like(exe: &std::path::Path, verb: &str, name: &str, kind: Kind, tool: &str) -> ShellCmd {
    let mut args = vec![verb.to_string()];
    if kind == Kind::Cask {
        args.push("--cask".into());
    }
    args.push(name.to_string());
    ShellCmd::new(
        exe.to_path_buf(),
        args.clone(),
        format!(
            "{tool} {verb}{} {name}",
            if kind == Kind::Cask { " --cask" } else { "" }
        ),
    )
}

fn zb_cmd(exe: &std::path::Path, verb: &str, name: &str) -> ShellCmd {
    ShellCmd::new(
        exe.to_path_buf(),
        vec![verb.into(), name.into()],
        format!("zb {verb} {name}"),
    )
}

fn trash_cmd(path: &std::path::Path) -> ShellCmd {
    let script = format!(
        r#"tell application "Finder" to delete POSIX file "{}""#,
        path.display()
    );
    ShellCmd::new(
        PathBuf::from("/usr/bin/osascript"),
        vec!["-e".into(), script],
        format!("osascript → move {} to Trash", path.display()),
    )
}
