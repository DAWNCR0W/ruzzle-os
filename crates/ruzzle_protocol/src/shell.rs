extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::tlv::{write_tlv, TlvReader};
use crate::ProtocolError;

/// TLV type for message type.
pub const TLV_MSG_TYPE: u16 = 1;
/// TLV type for status code.
pub const TLV_STATUS: u16 = 2;
/// TLV type for module name.
pub const TLV_MODULE: u16 = 3;
/// TLV type for help topic.
pub const TLV_TOPIC: u16 = 4;
/// TLV type for response text.
pub const TLV_TEXT: u16 = 5;
/// TLV type for filesystem paths.
pub const TLV_PATH: u16 = 6;
/// TLV type for puzzle slots.
pub const TLV_SLOT: u16 = 7;
/// TLV type for user names.
pub const TLV_USER: u16 = 8;
/// TLV type for write payloads.
pub const TLV_CONTENT: u16 = 9;
/// TLV type for copy/move source path.
pub const TLV_SRC: u16 = 10;
/// TLV type for copy/move destination path.
pub const TLV_DST: u16 = 11;
/// TLV type for command flag bits.
pub const TLV_FLAG: u16 = 12;
/// TLV type for raw argument strings.
pub const TLV_ARGS: u16 = 13;

/// Flag bit for recursive copy.
pub const FLAG_RECURSIVE: u8 = 0b0000_0001;
/// Flag bit for dry-run operations.
pub const FLAG_DRY_RUN: u8 = 0b0000_0001;
/// Flag bit for catalog verified-only filtering.
pub const FLAG_VERIFIED_ONLY: u8 = 0b0000_0001;
/// Flag bit reserved for hot-swap.
pub const FLAG_SWAP: u8 = 0b0000_0010;
/// Flag bit for process tree output.
pub const FLAG_TREE: u8 = 0b0000_0001;

/// Shell message: list processes.
pub const MSG_PS: u8 = 1;
/// Shell message: list modules.
pub const MSG_LSMOD: u8 = 2;
/// Shell message: start module.
pub const MSG_START: u8 = 3;
/// Shell message: stop module.
pub const MSG_STOP: u8 = 4;
/// Shell message: tail logs.
pub const MSG_LOG_TAIL: u8 = 5;
/// Shell message: help.
pub const MSG_HELP: u8 = 6;
/// Shell message: list available modules (catalog).
pub const MSG_CATALOG: u8 = 7;
/// Shell message: install a module from the catalog.
pub const MSG_INSTALL: u8 = 8;
/// Shell message: remove an installed module.
pub const MSG_REMOVE: u8 = 9;
/// Shell message: run first-boot setup.
pub const MSG_SETUP: u8 = 10;
/// Shell message: login.
pub const MSG_LOGIN: u8 = 11;
/// Shell message: logout.
pub const MSG_LOGOUT: u8 = 12;
/// Shell message: report current user.
pub const MSG_WHOAMI: u8 = 13;
/// Shell message: list users.
pub const MSG_USERS: u8 = 14;
/// Shell message: add user.
pub const MSG_USERADD: u8 = 15;
/// Shell message: print working directory.
pub const MSG_PWD: u8 = 16;
/// Shell message: list directory entries.
pub const MSG_LS: u8 = 17;
/// Shell message: change directory.
pub const MSG_CD: u8 = 18;
/// Shell message: create directory.
pub const MSG_MKDIR: u8 = 19;
/// Shell message: create empty file.
pub const MSG_TOUCH: u8 = 20;
/// Shell message: read file.
pub const MSG_CAT: u8 = 21;
/// Shell message: write file.
pub const MSG_WRITE: u8 = 22;
/// Shell message: edit file (vim-style).
pub const MSG_EDIT: u8 = 23;
/// Shell message: copy file or directory.
pub const MSG_CP: u8 = 24;
/// Shell message: move file or directory.
pub const MSG_MV: u8 = 25;
/// Shell message: create directory tree.
pub const MSG_MKDIRP: u8 = 26;
/// Shell message: remove recursively.
pub const MSG_RMR: u8 = 27;
/// Shell message: list puzzle slots.
pub const MSG_SLOTS: u8 = 28;
/// Shell message: plug module into slot.
pub const MSG_PLUG: u8 = 29;
/// Shell message: unplug module from slot.
pub const MSG_UNPLUG: u8 = 30;
/// Shell message: system info.
pub const MSG_SYSINFO: u8 = 31;
/// Shell message: remove file or directory.
pub const MSG_RM: u8 = 32;
/// Shell message: dependency graph.
pub const MSG_GRAPH: u8 = 33;
/// Shell message: piece health check.
pub const MSG_PIECE_CHECK: u8 = 34;
/// Shell message: ip command.
pub const MSG_IP: u8 = 35;
/// Shell message: route command.
pub const MSG_ROUTE: u8 = 36;
/// Shell message: mount command.
pub const MSG_MOUNT: u8 = 37;
/// Shell message: df command.
pub const MSG_DF: u8 = 38;
/// Shell message: du command.
pub const MSG_DU: u8 = 39;
/// Shell message: market scan command.
pub const MSG_MARKET_SCAN: u8 = 40;

/// Shell response status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellStatus {
    Ok,
    Failed,
}

impl ShellStatus {
    pub fn as_u8(self) -> u8 {
        match self {
            ShellStatus::Ok => 0,
            ShellStatus::Failed => 1,
        }
    }

    pub fn from_u8(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0 => Ok(ShellStatus::Ok),
            1 => Ok(ShellStatus::Failed),
            _ => Err(ProtocolError::InvalidValue("status")),
        }
    }
}

/// Shell command message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellCommand {
    Ps {
        tree: bool,
    },
    Lsmod,
    Start(String),
    Stop(String),
    LogTail,
    Help(Option<String>),
    Catalog {
        slot: Option<String>,
        verified_only: bool,
    },
    PieceCheck(String),
    Ip(Option<String>),
    Route(Option<String>),
    Mount(Option<String>),
    Df(Option<String>),
    Du(String),
    MarketScan,
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
    RmRecursive(String),
    Slots,
    Plug {
        slot: String,
        module: String,
        dry_run: bool,
        swap: bool,
    },
    Unplug(String),
    Graph,
    Sysinfo,
    Rm(String),
}

/// Shell response message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellResponse {
    pub status: ShellStatus,
    pub text: String,
}

/// Encodes a shell command into TLV bytes.
pub fn encode_command(command: &ShellCommand) -> Vec<u8> {
    let mut bytes = Vec::new();
    match command {
        ShellCommand::Ps { tree } => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_PS]);
            if *tree {
                write_tlv(&mut bytes, TLV_FLAG, &[FLAG_TREE]);
            }
        }
        ShellCommand::Lsmod => write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LSMOD]),
        ShellCommand::Start(module) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_START]);
            write_tlv(&mut bytes, TLV_MODULE, module.as_bytes());
        }
        ShellCommand::Stop(module) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_STOP]);
            write_tlv(&mut bytes, TLV_MODULE, module.as_bytes());
        }
        ShellCommand::LogTail => write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOG_TAIL]),
        ShellCommand::Help(topic) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_HELP]);
            if let Some(topic) = topic {
                write_tlv(&mut bytes, TLV_TOPIC, topic.as_bytes());
            }
        }
        ShellCommand::Catalog {
            slot,
            verified_only,
        } => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CATALOG]);
            if let Some(slot) = slot {
                write_tlv(&mut bytes, TLV_SLOT, slot.as_bytes());
            }
            if *verified_only {
                write_tlv(&mut bytes, TLV_FLAG, &[FLAG_VERIFIED_ONLY]);
            }
        }
        ShellCommand::PieceCheck(module) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_PIECE_CHECK]);
            write_tlv(&mut bytes, TLV_MODULE, module.as_bytes());
        }
        ShellCommand::Ip(args) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_IP]);
            if let Some(args) = args {
                write_tlv(&mut bytes, TLV_ARGS, args.as_bytes());
            }
        }
        ShellCommand::Route(args) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_ROUTE]);
            if let Some(args) = args {
                write_tlv(&mut bytes, TLV_ARGS, args.as_bytes());
            }
        }
        ShellCommand::Mount(args) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_MOUNT]);
            if let Some(args) = args {
                write_tlv(&mut bytes, TLV_ARGS, args.as_bytes());
            }
        }
        ShellCommand::Df(path) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_DF]);
            if let Some(path) = path {
                write_tlv(&mut bytes, TLV_PATH, path.as_bytes());
            }
        }
        ShellCommand::Du(path) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_DU]);
            write_tlv(&mut bytes, TLV_PATH, path.as_bytes());
        }
        ShellCommand::MarketScan => write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_MARKET_SCAN]),
        ShellCommand::Install(module) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_INSTALL]);
            write_tlv(&mut bytes, TLV_MODULE, module.as_bytes());
        }
        ShellCommand::Remove(module) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_REMOVE]);
            write_tlv(&mut bytes, TLV_MODULE, module.as_bytes());
        }
        ShellCommand::Setup => write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_SETUP]),
        ShellCommand::Login(user) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOGIN]);
            write_tlv(&mut bytes, TLV_USER, user.as_bytes());
        }
        ShellCommand::Logout => write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOGOUT]),
        ShellCommand::Whoami => write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_WHOAMI]),
        ShellCommand::Users => write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_USERS]),
        ShellCommand::UserAdd(user) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_USERADD]);
            write_tlv(&mut bytes, TLV_USER, user.as_bytes());
        }
        ShellCommand::Pwd => write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_PWD]),
        ShellCommand::Ls(path) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LS]);
            if let Some(path) = path {
                write_tlv(&mut bytes, TLV_PATH, path.as_bytes());
            }
        }
        ShellCommand::Cd(path) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CD]);
            write_tlv(&mut bytes, TLV_PATH, path.as_bytes());
        }
        ShellCommand::Mkdir(path) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_MKDIR]);
            write_tlv(&mut bytes, TLV_PATH, path.as_bytes());
        }
        ShellCommand::Touch(path) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_TOUCH]);
            write_tlv(&mut bytes, TLV_PATH, path.as_bytes());
        }
        ShellCommand::Cat(path) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CAT]);
            write_tlv(&mut bytes, TLV_PATH, path.as_bytes());
        }
        ShellCommand::Edit(path) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_EDIT]);
            write_tlv(&mut bytes, TLV_PATH, path.as_bytes());
        }
        ShellCommand::Cp { src, dst, recursive } => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CP]);
            write_tlv(&mut bytes, TLV_SRC, src.as_bytes());
            write_tlv(&mut bytes, TLV_DST, dst.as_bytes());
            if *recursive {
                write_tlv(&mut bytes, TLV_FLAG, &[FLAG_RECURSIVE]);
            }
        }
        ShellCommand::Mv { src, dst } => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_MV]);
            write_tlv(&mut bytes, TLV_SRC, src.as_bytes());
            write_tlv(&mut bytes, TLV_DST, dst.as_bytes());
        }
        ShellCommand::MkdirP(path) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_MKDIRP]);
            write_tlv(&mut bytes, TLV_PATH, path.as_bytes());
        }
        ShellCommand::Write { path, contents } => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_WRITE]);
            write_tlv(&mut bytes, TLV_PATH, path.as_bytes());
            write_tlv(&mut bytes, TLV_CONTENT, contents.as_bytes());
        }
        ShellCommand::Slots => write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_SLOTS]),
        ShellCommand::Plug {
            slot,
            module,
            dry_run,
            swap,
        } => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_PLUG]);
            write_tlv(&mut bytes, TLV_SLOT, slot.as_bytes());
            write_tlv(&mut bytes, TLV_MODULE, module.as_bytes());
            let mut flags = 0u8;
            if *dry_run {
                flags |= FLAG_DRY_RUN;
            }
            if *swap {
                flags |= FLAG_SWAP;
            }
            if flags != 0 {
                write_tlv(&mut bytes, TLV_FLAG, &[flags]);
            }
        }
        ShellCommand::Unplug(slot) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_UNPLUG]);
            write_tlv(&mut bytes, TLV_SLOT, slot.as_bytes());
        }
        ShellCommand::Graph => write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_GRAPH]),
        ShellCommand::Sysinfo => write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_SYSINFO]),
        ShellCommand::Rm(path) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_RM]);
            write_tlv(&mut bytes, TLV_PATH, path.as_bytes());
        }
        ShellCommand::RmRecursive(path) => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_RMR]);
            write_tlv(&mut bytes, TLV_PATH, path.as_bytes());
        }
    }
    bytes
}

/// Decodes a shell command from TLV bytes.
pub fn decode_command(bytes: &[u8]) -> Result<ShellCommand, ProtocolError> {
    let mut msg_type: Option<u8> = None;
    let mut module: Option<String> = None;
    let mut topic: Option<String> = None;
    let mut path: Option<String> = None;
    let mut slot: Option<String> = None;
    let mut user: Option<String> = None;
    let mut content: Option<String> = None;
    let mut src: Option<String> = None;
    let mut dst: Option<String> = None;
    let mut args: Option<String> = None;
    let mut flag: Option<u8> = None;

    let mut reader = TlvReader::new(bytes);
    while let Some(field) = reader.next()? {
        match field.tlv_type {
            TLV_MSG_TYPE => {
                if msg_type.is_some() {
                    return Err(ProtocolError::DuplicateField("msg_type"));
                }
                if field.value.len() != 1 {
                    return Err(ProtocolError::InvalidLength("msg_type"));
                }
                msg_type = Some(field.value[0]);
            }
            TLV_MODULE => {
                if module.is_some() {
                    return Err(ProtocolError::DuplicateField("module"));
                }
                module = Some(parse_string(field.value)?);
            }
            TLV_TOPIC => {
                if topic.is_some() {
                    return Err(ProtocolError::DuplicateField("topic"));
                }
                topic = Some(parse_string(field.value)?);
            }
            TLV_PATH => {
                if path.is_some() {
                    return Err(ProtocolError::DuplicateField("path"));
                }
                path = Some(parse_string(field.value)?);
            }
            TLV_SLOT => {
                if slot.is_some() {
                    return Err(ProtocolError::DuplicateField("slot"));
                }
                slot = Some(parse_string(field.value)?);
            }
            TLV_USER => {
                if user.is_some() {
                    return Err(ProtocolError::DuplicateField("user"));
                }
                user = Some(parse_string(field.value)?);
            }
            TLV_CONTENT => {
                if content.is_some() {
                    return Err(ProtocolError::DuplicateField("content"));
                }
                content = Some(parse_string(field.value)?);
            }
            TLV_SRC => {
                if src.is_some() {
                    return Err(ProtocolError::DuplicateField("src"));
                }
                src = Some(parse_string(field.value)?);
            }
            TLV_DST => {
                if dst.is_some() {
                    return Err(ProtocolError::DuplicateField("dst"));
                }
                dst = Some(parse_string(field.value)?);
            }
            TLV_ARGS => {
                if args.is_some() {
                    return Err(ProtocolError::DuplicateField("args"));
                }
                args = Some(parse_string(field.value)?);
            }
            TLV_FLAG => {
                if flag.is_some() {
                    return Err(ProtocolError::DuplicateField("flag"));
                }
                if field.value.len() != 1 {
                    return Err(ProtocolError::InvalidLength("flag"));
                }
                flag = Some(field.value[0]);
            }
            _ => {}
        }
    }

    let msg_type = msg_type.ok_or(ProtocolError::MissingField("msg_type"))?;
    match msg_type {
        MSG_PS => Ok(ShellCommand::Ps {
            tree: flag.map(|bits| bits & FLAG_TREE != 0).unwrap_or(false),
        }),
        MSG_LSMOD => Ok(ShellCommand::Lsmod),
        MSG_START => Ok(ShellCommand::Start(
            module.ok_or(ProtocolError::MissingField("module"))?,
        )),
        MSG_STOP => Ok(ShellCommand::Stop(
            module.ok_or(ProtocolError::MissingField("module"))?,
        )),
        MSG_LOG_TAIL => Ok(ShellCommand::LogTail),
        MSG_HELP => Ok(ShellCommand::Help(topic)),
        MSG_CATALOG => Ok(ShellCommand::Catalog {
            slot,
            verified_only: flag
                .map(|bits| bits & FLAG_VERIFIED_ONLY != 0)
                .unwrap_or(false),
        }),
        MSG_PIECE_CHECK => Ok(ShellCommand::PieceCheck(
            module.ok_or(ProtocolError::MissingField("module"))?,
        )),
        MSG_IP => Ok(ShellCommand::Ip(args)),
        MSG_ROUTE => Ok(ShellCommand::Route(args)),
        MSG_MOUNT => Ok(ShellCommand::Mount(args)),
        MSG_DF => Ok(ShellCommand::Df(path)),
        MSG_DU => Ok(ShellCommand::Du(
            path.ok_or(ProtocolError::MissingField("path"))?,
        )),
        MSG_MARKET_SCAN => Ok(ShellCommand::MarketScan),
        MSG_INSTALL => Ok(ShellCommand::Install(
            module.ok_or(ProtocolError::MissingField("module"))?,
        )),
        MSG_REMOVE => Ok(ShellCommand::Remove(
            module.ok_or(ProtocolError::MissingField("module"))?,
        )),
        MSG_SETUP => Ok(ShellCommand::Setup),
        MSG_LOGIN => Ok(ShellCommand::Login(
            user.ok_or(ProtocolError::MissingField("user"))?,
        )),
        MSG_LOGOUT => Ok(ShellCommand::Logout),
        MSG_WHOAMI => Ok(ShellCommand::Whoami),
        MSG_USERS => Ok(ShellCommand::Users),
        MSG_USERADD => Ok(ShellCommand::UserAdd(
            user.ok_or(ProtocolError::MissingField("user"))?,
        )),
        MSG_PWD => Ok(ShellCommand::Pwd),
        MSG_LS => Ok(ShellCommand::Ls(path)),
        MSG_CD => Ok(ShellCommand::Cd(
            path.ok_or(ProtocolError::MissingField("path"))?,
        )),
        MSG_MKDIR => Ok(ShellCommand::Mkdir(
            path.ok_or(ProtocolError::MissingField("path"))?,
        )),
        MSG_TOUCH => Ok(ShellCommand::Touch(
            path.ok_or(ProtocolError::MissingField("path"))?,
        )),
        MSG_CAT => Ok(ShellCommand::Cat(
            path.ok_or(ProtocolError::MissingField("path"))?,
        )),
        MSG_EDIT => Ok(ShellCommand::Edit(
            path.ok_or(ProtocolError::MissingField("path"))?,
        )),
        MSG_CP => Ok(ShellCommand::Cp {
            src: src.ok_or(ProtocolError::MissingField("src"))?,
            dst: dst.ok_or(ProtocolError::MissingField("dst"))?,
            recursive: flag.map(|bits| bits & FLAG_RECURSIVE != 0).unwrap_or(false),
        }),
        MSG_MV => Ok(ShellCommand::Mv {
            src: src.ok_or(ProtocolError::MissingField("src"))?,
            dst: dst.ok_or(ProtocolError::MissingField("dst"))?,
        }),
        MSG_MKDIRP => Ok(ShellCommand::MkdirP(
            path.ok_or(ProtocolError::MissingField("path"))?,
        )),
        MSG_WRITE => Ok(ShellCommand::Write {
            path: path.ok_or(ProtocolError::MissingField("path"))?,
            contents: content.ok_or(ProtocolError::MissingField("content"))?,
        }),
        MSG_SLOTS => Ok(ShellCommand::Slots),
        MSG_PLUG => Ok(ShellCommand::Plug {
            slot: slot.ok_or(ProtocolError::MissingField("slot"))?,
            module: module.ok_or(ProtocolError::MissingField("module"))?,
            dry_run: flag.map(|bits| bits & FLAG_DRY_RUN != 0).unwrap_or(false),
            swap: flag.map(|bits| bits & FLAG_SWAP != 0).unwrap_or(false),
        }),
        MSG_UNPLUG => Ok(ShellCommand::Unplug(
            slot.ok_or(ProtocolError::MissingField("slot"))?,
        )),
        MSG_GRAPH => Ok(ShellCommand::Graph),
        MSG_SYSINFO => Ok(ShellCommand::Sysinfo),
        MSG_RM => Ok(ShellCommand::Rm(
            path.ok_or(ProtocolError::MissingField("path"))?,
        )),
        MSG_RMR => Ok(ShellCommand::RmRecursive(
            path.ok_or(ProtocolError::MissingField("path"))?,
        )),
        other => Err(ProtocolError::UnknownMessageType(other)),
    }
}

/// Encodes a shell response into TLV bytes.
pub fn encode_response(response: &ShellResponse) -> Vec<u8> {
    let mut bytes = Vec::new();
    write_tlv(&mut bytes, TLV_STATUS, &[response.status.as_u8()]);
    write_tlv(&mut bytes, TLV_TEXT, response.text.as_bytes());
    bytes
}

/// Decodes a shell response from TLV bytes.
pub fn decode_response(bytes: &[u8]) -> Result<ShellResponse, ProtocolError> {
    let mut status: Option<ShellStatus> = None;
    let mut text: Option<String> = None;

    let mut reader = TlvReader::new(bytes);
    while let Some(field) = reader.next()? {
        match field.tlv_type {
            TLV_STATUS => {
                if status.is_some() {
                    return Err(ProtocolError::DuplicateField("status"));
                }
                if field.value.len() != 1 {
                    return Err(ProtocolError::InvalidLength("status"));
                }
                status = Some(ShellStatus::from_u8(field.value[0])?);
            }
            TLV_TEXT => {
                if text.is_some() {
                    return Err(ProtocolError::DuplicateField("text"));
                }
                text = Some(parse_string(field.value)?);
            }
            _ => {}
        }
    }

    Ok(ShellResponse {
        status: status.ok_or(ProtocolError::MissingField("status"))?,
        text: text.ok_or(ProtocolError::MissingField("text"))?,
    })
}

fn parse_string(value: &[u8]) -> Result<String, ProtocolError> {
    let text = core::str::from_utf8(value).map_err(|_| ProtocolError::InvalidUtf8)?;
    if text.is_empty() {
        return Err(ProtocolError::InvalidValue("string"));
    }
    Ok(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_start_command() {
        let cmd = ShellCommand::Start("net".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_help_command_with_topic() {
        let cmd = ShellCommand::Help(Some("ps".to_string()));
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_ps_command() {
        let cmd = ShellCommand::Ps { tree: false };
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_ps_command_tree() {
        let cmd = ShellCommand::Ps { tree: true };
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_lsmod_command() {
        let cmd = ShellCommand::Lsmod;
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_stop_command() {
        let cmd = ShellCommand::Stop("fs".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_catalog_command() {
        let cmd = ShellCommand::Catalog {
            slot: None,
            verified_only: false,
        };
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_catalog_command_with_filters() {
        let cmd = ShellCommand::Catalog {
            slot: Some("ruzzle.slot.net@1".to_string()),
            verified_only: true,
        };
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_piece_check_command() {
        let cmd = ShellCommand::PieceCheck("fs-service".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_ip_command() {
        let cmd = ShellCommand::Ip(Some("add eth0".to_string()));
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_ip_command_no_args() {
        let cmd = ShellCommand::Ip(None);
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_route_command() {
        let cmd = ShellCommand::Route(Some("add default eth0".to_string()));
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_route_command_no_args() {
        let cmd = ShellCommand::Route(None);
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_mount_command() {
        let cmd = ShellCommand::Mount(Some("memfs /mnt".to_string()));
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_mount_command_no_args() {
        let cmd = ShellCommand::Mount(None);
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_df_command() {
        let cmd = ShellCommand::Df(Some("/".to_string()));
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_df_command_no_path() {
        let cmd = ShellCommand::Df(None);
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_du_command() {
        let cmd = ShellCommand::Du("/etc".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_market_scan_command() {
        let cmd = ShellCommand::MarketScan;
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_install_command() {
        let cmd = ShellCommand::Install("fs-service".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_remove_command() {
        let cmd = ShellCommand::Remove("fs-service".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_log_tail_command() {
        let cmd = ShellCommand::LogTail;
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_help_command_without_topic() {
        let cmd = ShellCommand::Help(None);
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_setup_command() {
        let cmd = ShellCommand::Setup;
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_login_command() {
        let cmd = ShellCommand::Login("root".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_logout_command() {
        let cmd = ShellCommand::Logout;
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_whoami_command() {
        let cmd = ShellCommand::Whoami;
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_users_command() {
        let cmd = ShellCommand::Users;
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_useradd_command() {
        let cmd = ShellCommand::UserAdd("guest".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_pwd_command() {
        let cmd = ShellCommand::Pwd;
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_ls_command_with_path() {
        let cmd = ShellCommand::Ls(Some("/".to_string()));
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_ls_command_without_path() {
        let cmd = ShellCommand::Ls(None);
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_cd_command() {
        let cmd = ShellCommand::Cd("/home".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_mkdir_command() {
        let cmd = ShellCommand::Mkdir("/etc".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_touch_command() {
        let cmd = ShellCommand::Touch("/tmp/notes.txt".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_cat_command() {
        let cmd = ShellCommand::Cat("/etc/hostname".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_edit_command() {
        let cmd = ShellCommand::Edit("/etc/hostname".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_write_command() {
        let cmd = ShellCommand::Write {
            path: "/etc/hostname".to_string(),
            contents: "ruzzle".to_string(),
        };
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_cp_command() {
        let cmd = ShellCommand::Cp {
            src: "/etc/hostname".to_string(),
            dst: "/etc/hostname.bak".to_string(),
            recursive: false,
        };
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_cp_command_recursive() {
        let cmd = ShellCommand::Cp {
            src: "/etc".to_string(),
            dst: "/backup/etc".to_string(),
            recursive: true,
        };
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_mv_command() {
        let cmd = ShellCommand::Mv {
            src: "/etc/hostname".to_string(),
            dst: "/etc/hostname.old".to_string(),
        };
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_mkdirp_command() {
        let cmd = ShellCommand::MkdirP("/var/tmp".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_rmr_command() {
        let cmd = ShellCommand::RmRecursive("/var/tmp".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_slots_command() {
        let cmd = ShellCommand::Slots;
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_plug_command() {
        let cmd = ShellCommand::Plug {
            slot: "ruzzle.slot.console@1".to_string(),
            module: "console-service".to_string(),
            dry_run: false,
            swap: false,
        };
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_plug_command_dry_run() {
        let cmd = ShellCommand::Plug {
            slot: "ruzzle.slot.console@1".to_string(),
            module: "console-service".to_string(),
            dry_run: true,
            swap: false,
        };
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_plug_command_swap() {
        let cmd = ShellCommand::Plug {
            slot: "ruzzle.slot.console@1".to_string(),
            module: "console-service".to_string(),
            dry_run: false,
            swap: true,
        };
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_unplug_command() {
        let cmd = ShellCommand::Unplug("ruzzle.slot.console@1".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_graph_command() {
        let cmd = ShellCommand::Graph;
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_sysinfo_command() {
        let cmd = ShellCommand::Sysinfo;
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn encode_decode_rm_command() {
        let cmd = ShellCommand::Rm("/tmp/file".to_string());
        let bytes = encode_command(&cmd);
        let decoded = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn decode_command_rejects_missing_msg_type() {
        let result = decode_command(&[]);
        assert_eq!(result, Err(ProtocolError::MissingField("msg_type")));
    }

    #[test]
    fn decode_command_rejects_invalid_msg_type_length() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_PS, 0x00]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidLength("msg_type")));
    }

    #[test]
    fn decode_command_rejects_missing_module() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_START]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("module")));
    }

    #[test]
    fn decode_command_rejects_missing_module_for_install() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_INSTALL]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("module")));
    }

    #[test]
    fn decode_command_rejects_missing_module_for_remove() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_REMOVE]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("module")));
    }

    #[test]
    fn decode_command_rejects_missing_module_for_stop() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_STOP]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("module")));
    }

    #[test]
    fn decode_command_rejects_missing_module_for_piece_check() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_PIECE_CHECK]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("module")));
    }

    #[test]
    fn decode_command_rejects_missing_user_for_login() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOGIN]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("user")));
    }

    #[test]
    fn decode_command_rejects_missing_user_for_useradd() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_USERADD]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("user")));
    }

    #[test]
    fn decode_command_rejects_missing_path_for_cd() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CD]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("path")));
    }

    #[test]
    fn decode_command_rejects_missing_path_for_mkdir() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_MKDIR]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("path")));
    }

    #[test]
    fn decode_command_rejects_missing_path_for_touch() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_TOUCH]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("path")));
    }

    #[test]
    fn decode_command_rejects_missing_path_for_cat() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CAT]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("path")));
    }

    #[test]
    fn decode_command_rejects_missing_path_for_edit() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_EDIT]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("path")));
    }

    #[test]
    fn decode_command_rejects_missing_path_for_mkdirp() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_MKDIRP]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("path")));
    }

    #[test]
    fn decode_command_rejects_missing_path_for_write() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_WRITE]);
        write_tlv(&mut bytes, TLV_CONTENT, b"hello");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("path")));
    }

    #[test]
    fn decode_command_rejects_missing_src_for_cp() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CP]);
        write_tlv(&mut bytes, TLV_DST, b"/tmp/out");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("src")));
    }

    #[test]
    fn decode_command_rejects_missing_dst_for_cp() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CP]);
        write_tlv(&mut bytes, TLV_SRC, b"/tmp/in");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("dst")));
    }

    #[test]
    fn decode_command_rejects_missing_src_for_mv() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_MV]);
        write_tlv(&mut bytes, TLV_DST, b"/tmp/out");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("src")));
    }

    #[test]
    fn decode_command_rejects_missing_dst_for_mv() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_MV]);
        write_tlv(&mut bytes, TLV_SRC, b"/tmp/in");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("dst")));
    }

    #[test]
    fn decode_command_rejects_invalid_flag_length() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CP]);
        write_tlv(&mut bytes, TLV_SRC, b"/tmp/in");
        write_tlv(&mut bytes, TLV_DST, b"/tmp/out");
        write_tlv(&mut bytes, TLV_FLAG, &[0x01, 0x00]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidLength("flag")));
    }

    #[test]
    fn decode_command_defaults_flag_when_missing() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CP]);
        write_tlv(&mut bytes, TLV_SRC, b"/tmp/in");
        write_tlv(&mut bytes, TLV_DST, b"/tmp/out");
        let result = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(
            result,
            ShellCommand::Cp {
                src: "/tmp/in".to_string(),
                dst: "/tmp/out".to_string(),
                recursive: false
            }
        );
    }

    #[test]
    fn decode_command_rejects_duplicate_src() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CP]);
        write_tlv(&mut bytes, TLV_SRC, b"/tmp/in");
        write_tlv(&mut bytes, TLV_SRC, b"/tmp/in2");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("src")));
    }

    #[test]
    fn decode_command_rejects_duplicate_dst() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CP]);
        write_tlv(&mut bytes, TLV_DST, b"/tmp/out");
        write_tlv(&mut bytes, TLV_DST, b"/tmp/out2");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("dst")));
    }

    #[test]
    fn decode_command_rejects_duplicate_flag() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CP]);
        write_tlv(&mut bytes, TLV_SRC, b"/tmp/in");
        write_tlv(&mut bytes, TLV_DST, b"/tmp/out");
        write_tlv(&mut bytes, TLV_FLAG, &[0x00]);
        write_tlv(&mut bytes, TLV_FLAG, &[0x01]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("flag")));
    }

    #[test]
    fn decode_command_rejects_missing_content_for_write() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_WRITE]);
        write_tlv(&mut bytes, TLV_PATH, b"/tmp/file");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("content")));
    }

    #[test]
    fn decode_command_rejects_missing_slot_for_plug() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_PLUG]);
        write_tlv(&mut bytes, TLV_MODULE, b"console-service");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("slot")));
    }

    #[test]
    fn decode_command_rejects_missing_slot_for_unplug() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_UNPLUG]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("slot")));
    }

    #[test]
    fn decode_command_rejects_missing_path_for_rm() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_RM]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("path")));
    }

    #[test]
    fn decode_command_rejects_missing_path_for_rmr() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_RMR]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("path")));
    }

    #[test]
    fn decode_command_rejects_missing_path_for_du() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_DU]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("path")));
    }

    #[test]
    fn decode_command_rejects_unknown_type() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[0x42]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::UnknownMessageType(0x42)));
    }

    #[test]
    fn decode_command_rejects_duplicate_msg_type() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_PS]);
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_PS]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("msg_type")));
    }

    #[test]
    fn decode_command_rejects_duplicate_module() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_STOP]);
        write_tlv(&mut bytes, TLV_MODULE, b"fs");
        write_tlv(&mut bytes, TLV_MODULE, b"fs");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("module")));
    }

    #[test]
    fn decode_command_rejects_duplicate_path() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CD]);
        write_tlv(&mut bytes, TLV_PATH, b"/");
        write_tlv(&mut bytes, TLV_PATH, b"/etc");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("path")));
    }

    #[test]
    fn decode_command_rejects_duplicate_slot() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_UNPLUG]);
        write_tlv(&mut bytes, TLV_SLOT, b"ruzzle.slot.console@1");
        write_tlv(&mut bytes, TLV_SLOT, b"ruzzle.slot.shell@1");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("slot")));
    }

    #[test]
    fn decode_command_rejects_duplicate_user() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOGIN]);
        write_tlv(&mut bytes, TLV_USER, b"root");
        write_tlv(&mut bytes, TLV_USER, b"guest");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("user")));
    }

    #[test]
    fn decode_command_rejects_duplicate_content() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_WRITE]);
        write_tlv(&mut bytes, TLV_PATH, b"/tmp/file");
        write_tlv(&mut bytes, TLV_CONTENT, b"a");
        write_tlv(&mut bytes, TLV_CONTENT, b"b");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("content")));
    }

    #[test]
    fn decode_command_rejects_duplicate_topic() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_HELP]);
        write_tlv(&mut bytes, TLV_TOPIC, b"ps");
        write_tlv(&mut bytes, TLV_TOPIC, b"ps");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("topic")));
    }

    #[test]
    fn decode_command_rejects_duplicate_args() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_IP]);
        write_tlv(&mut bytes, TLV_ARGS, b"addr show");
        write_tlv(&mut bytes, TLV_ARGS, b"route show");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("args")));
    }

    #[test]
    fn decode_command_rejects_invalid_utf8_module() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_START]);
        write_tlv(&mut bytes, TLV_MODULE, &[0xFF]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_command_rejects_invalid_utf8_path() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CD]);
        write_tlv(&mut bytes, TLV_PATH, &[0xFF]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_command_rejects_invalid_utf8_src() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CP]);
        write_tlv(&mut bytes, TLV_SRC, &[0xFF]);
        write_tlv(&mut bytes, TLV_DST, b"/tmp/out");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_command_rejects_invalid_utf8_args() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_IP]);
        write_tlv(&mut bytes, TLV_ARGS, &[0xFF]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_command_rejects_invalid_utf8_dst() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_CP]);
        write_tlv(&mut bytes, TLV_SRC, b"/tmp/in");
        write_tlv(&mut bytes, TLV_DST, &[0xFF]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_command_rejects_invalid_utf8_slot() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_UNPLUG]);
        write_tlv(&mut bytes, TLV_SLOT, &[0xFF]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_command_rejects_invalid_utf8_user() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOGIN]);
        write_tlv(&mut bytes, TLV_USER, &[0xFF]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_command_rejects_invalid_utf8_content() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_WRITE]);
        write_tlv(&mut bytes, TLV_PATH, b"/tmp/file");
        write_tlv(&mut bytes, TLV_CONTENT, &[0xFF]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_command_rejects_empty_topic() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_HELP]);
        write_tlv(&mut bytes, TLV_TOPIC, &[]);
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidValue("string")));
    }

    #[test]
    fn decode_command_rejects_missing_module_for_plug() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_PLUG]);
        write_tlv(&mut bytes, TLV_SLOT, b"ruzzle.slot.console@1");
        let result = decode_command(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("module")));
    }

    #[test]
    fn decode_command_rejects_truncated_tlv() {
        let bytes = [TLV_MSG_TYPE as u8, 0x00, 0x01];
        let result = decode_command(&bytes);
        assert_eq!(
            result,
            Err(ProtocolError::Tlv(crate::tlv::TlvError::TruncatedHeader))
        );
    }

    #[test]
    fn decode_command_ignores_unknown_tlv() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_PS]);
        write_tlv(&mut bytes, 0x9999, b"ignored");
        let result = decode_command(&bytes).expect("decode should succeed");
        assert_eq!(result, ShellCommand::Ps { tree: false });
    }

    #[test]
    fn encode_decode_response_roundtrip() {
        let response = ShellResponse {
            status: ShellStatus::Ok,
            text: "ok".to_string(),
        };
        let bytes = encode_response(&response);
        let decoded = decode_response(&bytes).expect("decode should succeed");
        assert_eq!(decoded, response);
    }

    #[test]
    fn encode_decode_failed_response() {
        let response = ShellResponse {
            status: ShellStatus::Failed,
            text: "nope".to_string(),
        };
        let bytes = encode_response(&response);
        let decoded = decode_response(&bytes).expect("decode should succeed");
        assert_eq!(decoded, response);
    }

    #[test]
    fn decode_response_rejects_missing_fields() {
        let result = decode_response(&[]);
        assert_eq!(result, Err(ProtocolError::MissingField("status")));
    }

    #[test]
    fn decode_response_rejects_invalid_status_length() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_STATUS, &[0x01, 0x02]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidLength("status")));
    }

    #[test]
    fn decode_response_rejects_invalid_status_value() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_STATUS, &[0x03]);
        write_tlv(&mut bytes, TLV_TEXT, b"ok");
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidValue("status")));
    }

    #[test]
    fn decode_response_rejects_invalid_text() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_STATUS, &[0x00]);
        write_tlv(&mut bytes, TLV_TEXT, &[]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidValue("string")));
    }

    #[test]
    fn decode_response_rejects_duplicate_status() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_STATUS, &[0x00]);
        write_tlv(&mut bytes, TLV_STATUS, &[0x00]);
        write_tlv(&mut bytes, TLV_TEXT, b"ok");
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("status")));
    }

    #[test]
    fn decode_response_rejects_duplicate_text() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_STATUS, &[0x00]);
        write_tlv(&mut bytes, TLV_TEXT, b"ok");
        write_tlv(&mut bytes, TLV_TEXT, b"ok");
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("text")));
    }

    #[test]
    fn decode_response_rejects_missing_text() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_STATUS, &[0x00]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("text")));
    }

    #[test]
    fn decode_response_rejects_invalid_utf8_text() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_STATUS, &[0x00]);
        write_tlv(&mut bytes, TLV_TEXT, &[0xFF]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_response_rejects_truncated_tlv() {
        let bytes = [TLV_STATUS as u8, 0x00, 0x01];
        let result = decode_response(&bytes);
        assert_eq!(
            result,
            Err(ProtocolError::Tlv(crate::tlv::TlvError::TruncatedHeader))
        );
    }

    #[test]
    fn decode_response_ignores_unknown_tlv() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_STATUS, &[0x00]);
        write_tlv(&mut bytes, TLV_TEXT, b"ok");
        write_tlv(&mut bytes, 0x9999, b"ignored");
        let result = decode_response(&bytes).expect("decode should succeed");
        assert_eq!(
            result,
            ShellResponse {
                status: ShellStatus::Ok,
                text: "ok".to_string()
            }
        );
    }
}
