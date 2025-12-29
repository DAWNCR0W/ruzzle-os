#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ruzzle_protocol::shell as shell_protocol;

/// Commands supported by the TUI shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Ps,
    Lsmod,
    Start(String),
    Stop(String),
    LogTail,
    Help(Option<String>),
    Catalog,
    Install(String),
    Remove(String),
    Setup,
    Login(String),
    Logout,
    Whoami,
    Users,
    UserAdd(String),
    Pwd,
    Ls(Option<String>),
    Cd(String),
    Mkdir(String),
    Touch(String),
    Cat(String),
    Edit(String),
    Cp { src: String, dst: String, recursive: bool },
    Mv { src: String, dst: String },
    MkdirP(String),
    Write { path: String, contents: String },
    Rm(String),
    RmRecursive(String),
    Slots,
    Plug { slot: String, module: String },
    Unplug(String),
    Sysinfo,
    Unknown(String),
}

/// Lightweight module row for UI formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleRow {
    pub name: String,
    pub state: String,
    pub provides: Vec<String>,
}

/// Lightweight process row for UI formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRow {
    pub pid: Option<u32>,
    pub name: String,
    pub state: String,
}

/// Lightweight slot row for puzzle-board formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotRow {
    pub name: String,
    pub required: bool,
    pub provider: Option<String>,
}

/// Parses a shell command string into a structured command.
pub fn parse_command(input: &str) -> Command {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Command::Unknown("".to_string());
    }

    if trimmed == "ps" {
        return Command::Ps;
    }
    if trimmed == "lsmod" {
        return Command::Lsmod;
    }
    if trimmed == "catalog" {
        return Command::Catalog;
    }
    if trimmed == "setup" {
        return Command::Setup;
    }
    if trimmed == "logout" {
        return Command::Logout;
    }
    if trimmed == "whoami" {
        return Command::Whoami;
    }
    if trimmed == "users" {
        return Command::Users;
    }
    if trimmed == "pwd" {
        return Command::Pwd;
    }
    if trimmed == "slots" {
        return Command::Slots;
    }
    if trimmed == "sysinfo" {
        return Command::Sysinfo;
    }
    if trimmed == "log tail" {
        return Command::LogTail;
    }
    if trimmed.starts_with("help") {
        let rest = trimmed.strip_prefix("help").unwrap_or("").trim();
        if rest.is_empty() {
            return Command::Help(None);
        }
        return Command::Help(Some(rest.to_string()));
    }

    let mut parts = trimmed.split_whitespace();
    let cmd = parts.next().unwrap();

    match cmd {
        "start" => {
            let module = parts.collect::<Vec<&str>>().join(" ");
            if module.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Start(module)
            }
        }
        "login" => {
            let user = parts.collect::<Vec<&str>>().join(" ");
            if user.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Login(user)
            }
        }
        "useradd" => {
            let user = parts.collect::<Vec<&str>>().join(" ");
            if user.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::UserAdd(user)
            }
        }
        "stop" => {
            let module = parts.collect::<Vec<&str>>().join(" ");
            if module.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Stop(module)
            }
        }
        "ls" => {
            let path = parts.collect::<Vec<&str>>().join(" ");
            if path.is_empty() {
                Command::Ls(None)
            } else {
                Command::Ls(Some(path))
            }
        }
        "cd" => {
            let path = parts.collect::<Vec<&str>>().join(" ");
            if path.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Cd(path)
            }
        }
        "mkdir" => {
            let first = parts.next().unwrap_or("");
            if first == "-p" {
                let path = parts.collect::<Vec<&str>>().join(" ");
                if path.is_empty() {
                    Command::Unknown(trimmed.to_string())
                } else {
                    Command::MkdirP(path)
                }
            } else {
                let mut collected = Vec::new();
                if !first.is_empty() {
                    collected.push(first);
                }
                collected.extend(parts);
                let path = collected.join(" ");
                if path.is_empty() {
                    Command::Unknown(trimmed.to_string())
                } else {
                    Command::Mkdir(path)
                }
            }
        }
        "touch" => {
            let path = parts.collect::<Vec<&str>>().join(" ");
            if path.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Touch(path)
            }
        }
        "cat" => {
            let path = parts.collect::<Vec<&str>>().join(" ");
            if path.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Cat(path)
            }
        }
        "edit" | "vim" => {
            let path = parts.collect::<Vec<&str>>().join(" ");
            if path.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Edit(path)
            }
        }
        "rm" => {
            let first = parts.next().unwrap_or("");
            if first == "-r" || first == "-R" {
                let path = parts.collect::<Vec<&str>>().join(" ");
                if path.is_empty() {
                    Command::Unknown(trimmed.to_string())
                } else {
                    Command::RmRecursive(path)
                }
            } else {
                let mut collected = Vec::new();
                if !first.is_empty() {
                    collected.push(first);
                }
                collected.extend(parts);
                let path = collected.join(" ");
                if path.is_empty() {
                    Command::Unknown(trimmed.to_string())
                } else {
                    Command::Rm(path)
                }
            }
        }
        "write" => {
            let path = parts.next().unwrap_or("");
            let contents = parts.collect::<Vec<&str>>().join(" ");
            if path.is_empty() || contents.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Write {
                    path: path.to_string(),
                    contents,
                }
            }
        }
        "plug" => {
            let slot = parts.next().unwrap_or("");
            let module = parts.collect::<Vec<&str>>().join(" ");
            if slot.is_empty() || module.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Plug {
                    slot: slot.to_string(),
                    module,
                }
            }
        }
        "unplug" => {
            let slot = parts.collect::<Vec<&str>>().join(" ");
            if slot.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Unplug(slot)
            }
        }
        "install" => {
            let module = parts.collect::<Vec<&str>>().join(" ");
            if module.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Install(module)
            }
        }
        "cp" => {
            let first = parts.next().unwrap_or("");
            let mut recursive = false;
            let (src, dst) = if first == "-r" || first == "-R" {
                recursive = true;
                let src = parts.next().unwrap_or("");
                (src, parts.collect::<Vec<&str>>().join(" "))
            } else {
                (first, parts.collect::<Vec<&str>>().join(" "))
            };
            if src.is_empty() || dst.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Cp {
                    src: src.to_string(),
                    dst,
                    recursive,
                }
            }
        }
        "mv" => {
            let src = parts.next().unwrap_or("");
            let dst = parts.collect::<Vec<&str>>().join(" ");
            if src.is_empty() || dst.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Mv {
                    src: src.to_string(),
                    dst,
                }
            }
        }
        "remove" => {
            let module = parts.collect::<Vec<&str>>().join(" ");
            if module.is_empty() {
                Command::Unknown(trimmed.to_string())
            } else {
                Command::Remove(module)
            }
        }
        _ => Command::Unknown(trimmed.to_string()),
    }
}

/// Converts a parsed command into the IPC wire representation.
pub fn to_ipc(command: &Command) -> Option<shell_protocol::ShellCommand> {
    match command {
        Command::Ps => Some(shell_protocol::ShellCommand::Ps),
        Command::Lsmod => Some(shell_protocol::ShellCommand::Lsmod),
        Command::Start(name) => Some(shell_protocol::ShellCommand::Start(name.clone())),
        Command::Stop(name) => Some(shell_protocol::ShellCommand::Stop(name.clone())),
        Command::LogTail => Some(shell_protocol::ShellCommand::LogTail),
        Command::Help(topic) => Some(shell_protocol::ShellCommand::Help(topic.clone())),
        Command::Catalog => Some(shell_protocol::ShellCommand::Catalog),
        Command::Install(name) => Some(shell_protocol::ShellCommand::Install(name.clone())),
        Command::Remove(name) => Some(shell_protocol::ShellCommand::Remove(name.clone())),
        Command::Setup => Some(shell_protocol::ShellCommand::Setup),
        Command::Login(user) => Some(shell_protocol::ShellCommand::Login(user.clone())),
        Command::Logout => Some(shell_protocol::ShellCommand::Logout),
        Command::Whoami => Some(shell_protocol::ShellCommand::Whoami),
        Command::Users => Some(shell_protocol::ShellCommand::Users),
        Command::UserAdd(user) => Some(shell_protocol::ShellCommand::UserAdd(user.clone())),
        Command::Pwd => Some(shell_protocol::ShellCommand::Pwd),
        Command::Ls(path) => Some(shell_protocol::ShellCommand::Ls(path.clone())),
        Command::Cd(path) => Some(shell_protocol::ShellCommand::Cd(path.clone())),
        Command::Mkdir(path) => Some(shell_protocol::ShellCommand::Mkdir(path.clone())),
        Command::Touch(path) => Some(shell_protocol::ShellCommand::Touch(path.clone())),
        Command::Cat(path) => Some(shell_protocol::ShellCommand::Cat(path.clone())),
        Command::Edit(path) => Some(shell_protocol::ShellCommand::Edit(path.clone())),
        Command::Cp { src, dst, recursive } => Some(shell_protocol::ShellCommand::Cp {
            src: src.clone(),
            dst: dst.clone(),
            recursive: *recursive,
        }),
        Command::Mv { src, dst } => Some(shell_protocol::ShellCommand::Mv {
            src: src.clone(),
            dst: dst.clone(),
        }),
        Command::MkdirP(path) => Some(shell_protocol::ShellCommand::MkdirP(path.clone())),
        Command::Write { path, contents } => Some(shell_protocol::ShellCommand::Write {
            path: path.clone(),
            contents: contents.clone(),
        }),
        Command::Rm(path) => Some(shell_protocol::ShellCommand::Rm(path.clone())),
        Command::RmRecursive(path) => Some(shell_protocol::ShellCommand::RmRecursive(path.clone())),
        Command::Slots => Some(shell_protocol::ShellCommand::Slots),
        Command::Plug { slot, module } => Some(shell_protocol::ShellCommand::Plug {
            slot: slot.clone(),
            module: module.clone(),
        }),
        Command::Unplug(slot) => Some(shell_protocol::ShellCommand::Unplug(slot.clone())),
        Command::Sysinfo => Some(shell_protocol::ShellCommand::Sysinfo),
        Command::Unknown(_) => None,
    }
}

/// Converts an IPC command back into the local command enum.
pub fn from_ipc(command: shell_protocol::ShellCommand) -> Command {
    match command {
        shell_protocol::ShellCommand::Ps => Command::Ps,
        shell_protocol::ShellCommand::Lsmod => Command::Lsmod,
        shell_protocol::ShellCommand::Start(name) => Command::Start(name),
        shell_protocol::ShellCommand::Stop(name) => Command::Stop(name),
        shell_protocol::ShellCommand::LogTail => Command::LogTail,
        shell_protocol::ShellCommand::Help(topic) => Command::Help(topic),
        shell_protocol::ShellCommand::Catalog => Command::Catalog,
        shell_protocol::ShellCommand::Install(name) => Command::Install(name),
        shell_protocol::ShellCommand::Remove(name) => Command::Remove(name),
        shell_protocol::ShellCommand::Setup => Command::Setup,
        shell_protocol::ShellCommand::Login(user) => Command::Login(user),
        shell_protocol::ShellCommand::Logout => Command::Logout,
        shell_protocol::ShellCommand::Whoami => Command::Whoami,
        shell_protocol::ShellCommand::Users => Command::Users,
        shell_protocol::ShellCommand::UserAdd(user) => Command::UserAdd(user),
        shell_protocol::ShellCommand::Pwd => Command::Pwd,
        shell_protocol::ShellCommand::Ls(path) => Command::Ls(path),
        shell_protocol::ShellCommand::Cd(path) => Command::Cd(path),
        shell_protocol::ShellCommand::Mkdir(path) => Command::Mkdir(path),
        shell_protocol::ShellCommand::Touch(path) => Command::Touch(path),
        shell_protocol::ShellCommand::Cat(path) => Command::Cat(path),
        shell_protocol::ShellCommand::Edit(path) => Command::Edit(path),
        shell_protocol::ShellCommand::Cp { src, dst, recursive } => {
            Command::Cp { src, dst, recursive }
        }
        shell_protocol::ShellCommand::Mv { src, dst } => Command::Mv { src, dst },
        shell_protocol::ShellCommand::MkdirP(path) => Command::MkdirP(path),
        shell_protocol::ShellCommand::Write { path, contents } => {
            Command::Write { path, contents }
        }
        shell_protocol::ShellCommand::Rm(path) => Command::Rm(path),
        shell_protocol::ShellCommand::RmRecursive(path) => Command::RmRecursive(path),
        shell_protocol::ShellCommand::Slots => Command::Slots,
        shell_protocol::ShellCommand::Plug { slot, module } => Command::Plug { slot, module },
        shell_protocol::ShellCommand::Unplug(slot) => Command::Unplug(slot),
        shell_protocol::ShellCommand::Sysinfo => Command::Sysinfo,
    }
}

/// Formats the help text shown by the shell.
pub fn format_help() -> String {
    let mut out = String::new();
    out.push_str("commands:\n");
    out.push_str("  ps\n");
    out.push_str("  lsmod\n");
    out.push_str("  start <module>\n");
    out.push_str("  stop <module>\n");
    out.push_str("  catalog\n");
    out.push_str("  install <module>\n");
    out.push_str("  remove <module>\n");
    out.push_str("  setup\n");
    out.push_str("  login <user>\n");
    out.push_str("  logout\n");
    out.push_str("  whoami\n");
    out.push_str("  users\n");
    out.push_str("  useradd <user>\n");
    out.push_str("  pwd\n");
    out.push_str("  ls [path]\n");
    out.push_str("  cd <path>\n");
    out.push_str("  mkdir <path>\n");
    out.push_str("  mkdir -p <path>\n");
    out.push_str("  touch <path>\n");
    out.push_str("  cat <path>\n");
    out.push_str("  edit <path>\n");
    out.push_str("  vim <path>\n");
    out.push_str("  cp <src> <dst>\n");
    out.push_str("  cp -r <src> <dst>\n");
    out.push_str("  mv <src> <dst>\n");
    out.push_str("  write <path> <text>\n");
    out.push_str("  rm <path>\n");
    out.push_str("  rm -r <path>\n");
    out.push_str("  slots\n");
    out.push_str("  plug <slot> <module>\n");
    out.push_str("  unplug <slot>\n");
    out.push_str("  sysinfo\n");
    out.push_str("  log tail\n");
    out.push_str("  help [command]\n");
    out
}

/// Formats the available module catalog.
pub fn format_catalog(rows: &[ModuleRow]) -> String {
    let mut out = String::new();
    out.push_str("catalog:\n");
    if rows.is_empty() {
        out.push_str("  <none>\n");
        return out;
    }
    for row in rows {
        let provides = join_list(&row.provides);
        out.push_str("  ");
        out.push_str(&row.name);
        out.push_str(" provides: ");
        out.push_str(&provides);
        out.push('\n');
    }
    out
}

/// Formats the puzzle slot board.
pub fn format_slots(rows: &[SlotRow]) -> String {
    let mut out = String::new();
    out.push_str("slots:\n");
    if rows.is_empty() {
        out.push_str("  <none>\n");
        return out;
    }
    for row in rows {
        out.push_str("  ");
        out.push_str(&row.name);
        out.push_str(" [");
        out.push_str(if row.required { "required" } else { "optional" });
        out.push_str("] -> ");
        match &row.provider {
            Some(provider) => out.push_str(provider),
            None => out.push_str("<empty>"),
        }
        out.push('\n');
    }
    out
}

/// Formats a module list into a user-friendly table.
pub fn format_modules(rows: &[ModuleRow]) -> String {
    let mut out = String::new();
    out.push_str("modules:\n");
    if rows.is_empty() {
        out.push_str("  <none>\n");
        return out;
    }
    for row in rows {
        let provides = join_list(&row.provides);
        out.push_str("  ");
        out.push_str(&row.name);
        out.push_str(" [");
        out.push_str(&row.state);
        out.push_str("] provides: ");
        out.push_str(&provides);
        out.push('\n');
    }
    out
}

/// Formats a process list into a user-friendly table.
pub fn format_processes(rows: &[ProcessRow]) -> String {
    let mut out = String::new();
    out.push_str("processes:\n");
    if rows.is_empty() {
        out.push_str("  <none>\n");
        return out;
    }
    for row in rows {
        out.push_str("  ");
        match row.pid {
            Some(pid) => out.push_str(&pid.to_string()),
            None => out.push_str("-"),
        }
        out.push(' ');
        out.push_str(&row.name);
        out.push_str(" [");
        out.push_str(&row.state);
        out.push_str("]\n");
    }
    out
}

/// Formats the empty log tail response.
pub fn format_log_tail_empty() -> String {
    "log tail: no buffered logs available".to_string()
}

/// Formats an unknown command response.
pub fn format_unknown_command(raw: &str) -> String {
    let mut out = String::new();
    out.push_str("unknown command: ");
    out.push_str(raw);
    out
}

fn join_list(values: &[String]) -> String {
    if values.is_empty() {
        return "-".to_string();
    }
    let mut out = String::new();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_commands() {
        assert_eq!(parse_command("ps"), Command::Ps);
        assert_eq!(parse_command("lsmod"), Command::Lsmod);
        assert_eq!(parse_command("catalog"), Command::Catalog);
        assert_eq!(parse_command("setup"), Command::Setup);
        assert_eq!(parse_command("logout"), Command::Logout);
        assert_eq!(parse_command("whoami"), Command::Whoami);
        assert_eq!(parse_command("users"), Command::Users);
        assert_eq!(parse_command("pwd"), Command::Pwd);
        assert_eq!(parse_command("slots"), Command::Slots);
        assert_eq!(parse_command("sysinfo"), Command::Sysinfo);
        assert_eq!(parse_command("log tail"), Command::LogTail);
        assert_eq!(parse_command("help"), Command::Help(None));
        assert_eq!(
            parse_command("help ps"),
            Command::Help(Some("ps".to_string()))
        );
    }

    #[test]
    fn parse_start_and_stop_commands() {
        assert_eq!(
            parse_command("start net-service"),
            Command::Start("net-service".to_string())
        );
        assert_eq!(
            parse_command("stop gpu-service"),
            Command::Stop("gpu-service".to_string())
        );
        assert_eq!(
            parse_command("login root"),
            Command::Login("root".to_string())
        );
        assert_eq!(
            parse_command("useradd guest"),
            Command::UserAdd("guest".to_string())
        );
        assert_eq!(
            parse_command("install fs-service"),
            Command::Install("fs-service".to_string())
        );
        assert_eq!(
            parse_command("remove fs-service"),
            Command::Remove("fs-service".to_string())
        );
    }

    #[test]
    fn parse_filesystem_and_slot_commands() {
        assert_eq!(parse_command("ls"), Command::Ls(None));
        assert_eq!(
            parse_command("ls /etc"),
            Command::Ls(Some("/etc".to_string()))
        );
        assert_eq!(parse_command("cd /home"), Command::Cd("/home".to_string()));
        assert_eq!(
            parse_command("mkdir /tmp"),
            Command::Mkdir("/tmp".to_string())
        );
        assert_eq!(
            parse_command("mkdir -p /var/tmp"),
            Command::MkdirP("/var/tmp".to_string())
        );
        assert_eq!(
            parse_command("touch /tmp/a"),
            Command::Touch("/tmp/a".to_string())
        );
        assert_eq!(
            parse_command("cat /etc/hostname"),
            Command::Cat("/etc/hostname".to_string())
        );
        assert_eq!(
            parse_command("edit /etc/hostname"),
            Command::Edit("/etc/hostname".to_string())
        );
        assert_eq!(
            parse_command("vim /etc/hostname"),
            Command::Edit("/etc/hostname".to_string())
        );
        assert_eq!(
            parse_command("write /etc/hostname ruzzle"),
            Command::Write {
                path: "/etc/hostname".to_string(),
                contents: "ruzzle".to_string()
            }
        );
        assert_eq!(
            parse_command("rm /tmp/a"),
            Command::Rm("/tmp/a".to_string())
        );
        assert_eq!(
            parse_command("rm -r /var/tmp"),
            Command::RmRecursive("/var/tmp".to_string())
        );
        assert_eq!(
            parse_command("cp /etc/hostname /etc/hostname.bak"),
            Command::Cp {
                src: "/etc/hostname".to_string(),
                dst: "/etc/hostname.bak".to_string(),
                recursive: false
            }
        );
        assert_eq!(
            parse_command("cp -r /etc /backup/etc"),
            Command::Cp {
                src: "/etc".to_string(),
                dst: "/backup/etc".to_string(),
                recursive: true
            }
        );
        assert_eq!(
            parse_command("mv /etc/hostname /etc/hostname.old"),
            Command::Mv {
                src: "/etc/hostname".to_string(),
                dst: "/etc/hostname.old".to_string()
            }
        );
        assert_eq!(
            parse_command("plug ruzzle.slot.console console-service"),
            Command::Plug {
                slot: "ruzzle.slot.console".to_string(),
                module: "console-service".to_string()
            }
        );
        assert_eq!(
            parse_command("unplug ruzzle.slot.console"),
            Command::Unplug("ruzzle.slot.console".to_string())
        );
    }

    #[test]
    fn parse_unknown_or_incomplete_commands() {
        assert_eq!(parse_command(""), Command::Unknown("".to_string()));
        assert_eq!(parse_command("start"), Command::Unknown("start".to_string()));
        assert_eq!(parse_command("stop"), Command::Unknown("stop".to_string()));
        assert_eq!(parse_command("login"), Command::Unknown("login".to_string()));
        assert_eq!(parse_command("useradd"), Command::Unknown("useradd".to_string()));
        assert_eq!(parse_command("cd"), Command::Unknown("cd".to_string()));
        assert_eq!(parse_command("mkdir"), Command::Unknown("mkdir".to_string()));
        assert_eq!(parse_command("mkdir -p"), Command::Unknown("mkdir -p".to_string()));
        assert_eq!(parse_command("touch"), Command::Unknown("touch".to_string()));
        assert_eq!(parse_command("cat"), Command::Unknown("cat".to_string()));
        assert_eq!(parse_command("edit"), Command::Unknown("edit".to_string()));
        assert_eq!(parse_command("vim"), Command::Unknown("vim".to_string()));
        assert_eq!(parse_command("rm"), Command::Unknown("rm".to_string()));
        assert_eq!(parse_command("rm -r"), Command::Unknown("rm -r".to_string()));
        assert_eq!(parse_command("cp"), Command::Unknown("cp".to_string()));
        assert_eq!(parse_command("cp /etc/hostname"), Command::Unknown("cp /etc/hostname".to_string()));
        assert_eq!(parse_command("mv"), Command::Unknown("mv".to_string()));
        assert_eq!(parse_command("mv /etc/hostname"), Command::Unknown("mv /etc/hostname".to_string()));
        assert_eq!(parse_command("write"), Command::Unknown("write".to_string()));
        assert_eq!(parse_command("write /etc/hostname"), Command::Unknown("write /etc/hostname".to_string()));
        assert_eq!(parse_command("plug"), Command::Unknown("plug".to_string()));
        assert_eq!(parse_command("plug slot"), Command::Unknown("plug slot".to_string()));
        assert_eq!(parse_command("unplug"), Command::Unknown("unplug".to_string()));
        assert_eq!(parse_command("install"), Command::Unknown("install".to_string()));
        assert_eq!(parse_command("remove"), Command::Unknown("remove".to_string()));
        assert_eq!(parse_command("foo"), Command::Unknown("foo".to_string()));
    }

    #[test]
    fn ipc_conversions_roundtrip() {
        let cmd = Command::Start("fs".to_string());
        let wire = to_ipc(&cmd).expect("should map");
        let parsed = from_ipc(wire);
        assert_eq!(parsed, cmd);
    }

    #[test]
    fn ipc_conversion_drops_unknown() {
        let cmd = Command::Unknown("wat".to_string());
        assert_eq!(to_ipc(&cmd), None);
    }

    #[test]
    fn ipc_conversion_maps_all_commands() {
        assert_eq!(to_ipc(&Command::Ps), Some(shell_protocol::ShellCommand::Ps));
        assert_eq!(
            to_ipc(&Command::Lsmod),
            Some(shell_protocol::ShellCommand::Lsmod)
        );
        assert_eq!(
            to_ipc(&Command::Stop("fs".to_string())),
            Some(shell_protocol::ShellCommand::Stop("fs".to_string()))
        );
        assert_eq!(
            to_ipc(&Command::LogTail),
            Some(shell_protocol::ShellCommand::LogTail)
        );
        assert_eq!(
            to_ipc(&Command::Help(None)),
            Some(shell_protocol::ShellCommand::Help(None))
        );
        assert_eq!(
            to_ipc(&Command::Catalog),
            Some(shell_protocol::ShellCommand::Catalog)
        );
        assert_eq!(
            to_ipc(&Command::Install("fs".to_string())),
            Some(shell_protocol::ShellCommand::Install("fs".to_string()))
        );
        assert_eq!(
            to_ipc(&Command::Remove("fs".to_string())),
            Some(shell_protocol::ShellCommand::Remove("fs".to_string()))
        );
        assert_eq!(
            to_ipc(&Command::Setup),
            Some(shell_protocol::ShellCommand::Setup)
        );
        assert_eq!(
            to_ipc(&Command::Login("root".to_string())),
            Some(shell_protocol::ShellCommand::Login("root".to_string()))
        );
        assert_eq!(
            to_ipc(&Command::Logout),
            Some(shell_protocol::ShellCommand::Logout)
        );
        assert_eq!(
            to_ipc(&Command::Whoami),
            Some(shell_protocol::ShellCommand::Whoami)
        );
        assert_eq!(
            to_ipc(&Command::Users),
            Some(shell_protocol::ShellCommand::Users)
        );
        assert_eq!(
            to_ipc(&Command::UserAdd("guest".to_string())),
            Some(shell_protocol::ShellCommand::UserAdd("guest".to_string()))
        );
        assert_eq!(
            to_ipc(&Command::Pwd),
            Some(shell_protocol::ShellCommand::Pwd)
        );
        assert_eq!(
            to_ipc(&Command::Ls(None)),
            Some(shell_protocol::ShellCommand::Ls(None))
        );
        assert_eq!(
            to_ipc(&Command::Cd("/".to_string())),
            Some(shell_protocol::ShellCommand::Cd("/".to_string()))
        );
        assert_eq!(
            to_ipc(&Command::Mkdir("/etc".to_string())),
            Some(shell_protocol::ShellCommand::Mkdir("/etc".to_string()))
        );
        assert_eq!(
            to_ipc(&Command::Touch("/tmp/a".to_string())),
            Some(shell_protocol::ShellCommand::Touch("/tmp/a".to_string()))
        );
        assert_eq!(
            to_ipc(&Command::Cat("/etc/hostname".to_string())),
            Some(shell_protocol::ShellCommand::Cat("/etc/hostname".to_string()))
        );
        assert_eq!(
            to_ipc(&Command::Edit("/etc/hostname".to_string())),
            Some(shell_protocol::ShellCommand::Edit("/etc/hostname".to_string()))
        );
        assert_eq!(
            to_ipc(&Command::Write {
                path: "/etc/hostname".to_string(),
                contents: "ruzzle".to_string()
            }),
            Some(shell_protocol::ShellCommand::Write {
                path: "/etc/hostname".to_string(),
                contents: "ruzzle".to_string()
            })
        );
        assert_eq!(
            to_ipc(&Command::Cp {
                src: "/etc/hostname".to_string(),
                dst: "/etc/hostname.bak".to_string(),
                recursive: false
            }),
            Some(shell_protocol::ShellCommand::Cp {
                src: "/etc/hostname".to_string(),
                dst: "/etc/hostname.bak".to_string(),
                recursive: false
            })
        );
        assert_eq!(
            to_ipc(&Command::Mv {
                src: "/etc/hostname".to_string(),
                dst: "/etc/hostname.old".to_string()
            }),
            Some(shell_protocol::ShellCommand::Mv {
                src: "/etc/hostname".to_string(),
                dst: "/etc/hostname.old".to_string()
            })
        );
        assert_eq!(
            to_ipc(&Command::MkdirP("/var/tmp".to_string())),
            Some(shell_protocol::ShellCommand::MkdirP("/var/tmp".to_string()))
        );
        assert_eq!(
            to_ipc(&Command::Rm("/tmp/a".to_string())),
            Some(shell_protocol::ShellCommand::Rm("/tmp/a".to_string()))
        );
        assert_eq!(
            to_ipc(&Command::RmRecursive("/var/tmp".to_string())),
            Some(shell_protocol::ShellCommand::RmRecursive("/var/tmp".to_string()))
        );
        assert_eq!(
            to_ipc(&Command::Slots),
            Some(shell_protocol::ShellCommand::Slots)
        );
        assert_eq!(
            to_ipc(&Command::Plug {
                slot: "ruzzle.slot.console".to_string(),
                module: "console-service".to_string()
            }),
            Some(shell_protocol::ShellCommand::Plug {
                slot: "ruzzle.slot.console".to_string(),
                module: "console-service".to_string()
            })
        );
        assert_eq!(
            to_ipc(&Command::Unplug("ruzzle.slot.console".to_string())),
            Some(shell_protocol::ShellCommand::Unplug(
                "ruzzle.slot.console".to_string()
            ))
        );
        assert_eq!(
            to_ipc(&Command::Sysinfo),
            Some(shell_protocol::ShellCommand::Sysinfo)
        );
    }

    #[test]
    fn from_ipc_maps_all_commands() {
        assert_eq!(from_ipc(shell_protocol::ShellCommand::Ps), Command::Ps);
        assert_eq!(from_ipc(shell_protocol::ShellCommand::Lsmod), Command::Lsmod);
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Stop("fs".to_string())),
            Command::Stop("fs".to_string())
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::LogTail),
            Command::LogTail
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Help(None)),
            Command::Help(None)
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Catalog),
            Command::Catalog
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Install("fs".to_string())),
            Command::Install("fs".to_string())
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Remove("fs".to_string())),
            Command::Remove("fs".to_string())
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Setup),
            Command::Setup
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Login("root".to_string())),
            Command::Login("root".to_string())
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Logout),
            Command::Logout
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Whoami),
            Command::Whoami
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Users),
            Command::Users
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::UserAdd("guest".to_string())),
            Command::UserAdd("guest".to_string())
        );
        assert_eq!(from_ipc(shell_protocol::ShellCommand::Pwd), Command::Pwd);
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Ls(Some("/".to_string()))),
            Command::Ls(Some("/".to_string()))
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Cd("/".to_string())),
            Command::Cd("/".to_string())
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Mkdir("/etc".to_string())),
            Command::Mkdir("/etc".to_string())
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Touch("/tmp/a".to_string())),
            Command::Touch("/tmp/a".to_string())
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Cat("/etc/hostname".to_string())),
            Command::Cat("/etc/hostname".to_string())
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Edit(
                "/etc/hostname".to_string()
            )),
            Command::Edit("/etc/hostname".to_string())
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Write {
                path: "/etc/hostname".to_string(),
                contents: "ruzzle".to_string()
            }),
            Command::Write {
                path: "/etc/hostname".to_string(),
                contents: "ruzzle".to_string()
            }
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Cp {
                src: "/etc/hostname".to_string(),
                dst: "/etc/hostname.bak".to_string(),
                recursive: false
            }),
            Command::Cp {
                src: "/etc/hostname".to_string(),
                dst: "/etc/hostname.bak".to_string(),
                recursive: false
            }
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Mv {
                src: "/etc/hostname".to_string(),
                dst: "/etc/hostname.old".to_string()
            }),
            Command::Mv {
                src: "/etc/hostname".to_string(),
                dst: "/etc/hostname.old".to_string()
            }
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::MkdirP("/var/tmp".to_string())),
            Command::MkdirP("/var/tmp".to_string())
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Rm("/tmp/a".to_string())),
            Command::Rm("/tmp/a".to_string())
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::RmRecursive("/var/tmp".to_string())),
            Command::RmRecursive("/var/tmp".to_string())
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Slots),
            Command::Slots
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Plug {
                slot: "ruzzle.slot.console".to_string(),
                module: "console-service".to_string()
            }),
            Command::Plug {
                slot: "ruzzle.slot.console".to_string(),
                module: "console-service".to_string()
            }
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Unplug(
                "ruzzle.slot.console".to_string()
            )),
            Command::Unplug("ruzzle.slot.console".to_string())
        );
        assert_eq!(
            from_ipc(shell_protocol::ShellCommand::Sysinfo),
            Command::Sysinfo
        );
    }

    #[test]
    fn format_help_includes_commands() {
        let help = format_help();
        assert!(help.contains("ps"));
        assert!(help.contains("lsmod"));
        assert!(help.contains("catalog"));
        assert!(help.contains("install"));
        assert!(help.contains("remove"));
        assert!(help.contains("setup"));
        assert!(help.contains("login"));
        assert!(help.contains("pwd"));
        assert!(help.contains("slots"));
        assert!(help.contains("edit"));
        assert!(help.contains("cp"));
        assert!(help.contains("mv"));
        assert!(help.contains("rm"));
        assert!(help.contains("help"));
    }

    #[test]
    fn format_modules_handles_empty() {
        let output = format_modules(&[]);
        assert!(output.contains("<none>"));
    }

    #[test]
    fn format_modules_renders_rows() {
        let rows = vec![ModuleRow {
            name: "console-service".to_string(),
            state: "running".to_string(),
            provides: vec!["ruzzle.console".to_string()],
        }];
        let output = format_modules(&rows);
        assert!(output.contains("console-service"));
        assert!(output.contains("running"));
        assert!(output.contains("ruzzle.console"));
    }

    #[test]
    fn format_modules_renders_multiple_provides() {
        let rows = vec![ModuleRow {
            name: "init".to_string(),
            state: "running".to_string(),
            provides: vec!["ruzzle.console".to_string(), "ruzzle.shell".to_string()],
        }];
        let output = format_modules(&rows);
        assert!(output.contains("ruzzle.console, ruzzle.shell"));
    }

    #[test]
    fn format_modules_renders_empty_provides() {
        let rows = vec![ModuleRow {
            name: "init".to_string(),
            state: "running".to_string(),
            provides: vec![],
        }];
        let output = format_modules(&rows);
        assert!(output.contains("provides: -"));
    }

    #[test]
    fn format_catalog_handles_empty() {
        let output = format_catalog(&[]);
        assert!(output.contains("catalog:"));
        assert!(output.contains("<none>"));
    }

    #[test]
    fn format_catalog_renders_rows() {
        let rows = vec![ModuleRow {
            name: "fs-service".to_string(),
            state: "available".to_string(),
            provides: vec!["ruzzle.fs".to_string()],
        }];
        let output = format_catalog(&rows);
        assert!(output.contains("fs-service"));
        assert!(!output.contains("available"));
        assert!(output.contains("ruzzle.fs"));
    }

    #[test]
    fn format_slots_handles_empty() {
        let output = format_slots(&[]);
        assert!(output.contains("slots:"));
        assert!(output.contains("<none>"));
    }

    #[test]
    fn format_slots_renders_rows() {
        let rows = vec![SlotRow {
            name: "ruzzle.slot.console".to_string(),
            required: true,
            provider: Some("console-service".to_string()),
        }];
        let output = format_slots(&rows);
        assert!(output.contains("ruzzle.slot.console"));
        assert!(output.contains("required"));
        assert!(output.contains("console-service"));
    }

    #[test]
    fn format_slots_renders_optional_empty() {
        let rows = vec![SlotRow {
            name: "ruzzle.slot.net".to_string(),
            required: false,
            provider: None,
        }];
        let output = format_slots(&rows);
        assert!(output.contains("ruzzle.slot.net"));
        assert!(output.contains("optional"));
        assert!(output.contains("<empty>"));
    }

    #[test]
    fn format_processes_handles_empty() {
        let output = format_processes(&[]);
        assert!(output.contains("<none>"));
    }

    #[test]
    fn format_processes_renders_rows() {
        let rows = vec![ProcessRow {
            pid: Some(7),
            name: "init".to_string(),
            state: "running".to_string(),
        }];
        let output = format_processes(&rows);
        assert!(output.contains("7"));
        assert!(output.contains("init"));
        assert!(output.contains("running"));
    }

    #[test]
    fn format_processes_renders_unknown_pid() {
        let rows = vec![ProcessRow {
            pid: None,
            name: "init".to_string(),
            state: "running".to_string(),
        }];
        let output = format_processes(&rows);
        assert!(output.contains(" - init"));
    }

    #[test]
    fn format_log_tail_is_stable() {
        assert_eq!(format_log_tail_empty(), "log tail: no buffered logs available");
    }

    #[test]
    fn format_unknown_command_includes_input() {
        let output = format_unknown_command("wat");
        assert!(output.contains("wat"));
    }
}
