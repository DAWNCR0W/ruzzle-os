use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use kernel_core::{parse_initramfs, parse_module_bundle, parse_module_manifest, ModuleManifest};
use user_file_manager::FileManager;
use user_fs_service::{FileSystem, FsError};
use user_puzzle_board::{BoardError, PuzzleBoard, PuzzleSlot};
use user_session_service::SessionManager;
use user_settings_service::SystemSettings;
use user_setup_wizard::{run_first_boot, SetupPlan, SetupError};
use user_sysinfo_service::{build_system_info, format_system_info};
use user_text_editor::TextBuffer;
use user_tui_shell::{
    format_catalog, format_help, format_log_tail_empty, format_modules, format_processes,
    format_slots, format_unknown_command, parse_command, Command, ModuleRow, ProcessRow, SlotRow,
};
use user_user_service::{default_home_dir, UserManager};

use crate::{console, kprint, kprintln};

#[derive(Debug, Clone)]
struct ModuleEntry {
    name: String,
    manifest: Option<ModuleManifest>,
    running: bool,
}

#[derive(Debug, Clone)]
struct CatalogEntry {
    name: String,
    manifest: ModuleManifest,
}

pub fn run(initramfs: Option<&[u8]>) -> ! {
    let mut state = ShellState::new(initramfs);
    kprintln!("Ruzzle OS shell ready. Type 'help' for commands.");
    loop {
        kprint!("ruzzle> ");
        let line = read_line();
        let command = parse_command(&line);
        state.handle(command, &line);
    }
}

struct ShellState {
    modules: Vec<ModuleEntry>,
    catalog: Vec<CatalogEntry>,
    fs: FileSystem,
    file_manager: FileManager,
    users: UserManager,
    session: SessionManager,
    settings: SystemSettings,
    board: PuzzleBoard,
}

impl ShellState {
    fn new(initramfs: Option<&[u8]>) -> Self {
        let (modules, catalog) = build_modules(initramfs);
        let fs = FileSystem::new();
        let file_manager = FileManager::new();
        let users = UserManager::new();
        let session = SessionManager::new();
        let settings = SystemSettings::new_defaults();
        let board = build_puzzle_board(&modules);
        let mut state = Self {
            modules,
            catalog,
            fs,
            file_manager,
            users,
            session,
            settings,
            board,
        };
        state.ensure_setup();
        state.ensure_base_profile();
        state
    }

    fn handle(&mut self, command: Command, raw: &str) {
        if command_requires_login(&command) && self.require_login().is_none() {
            return;
        }
        match command {
            Command::Ps => self.print_running(),
            Command::Lsmod => self.print_modules(),
            Command::Start(name) => self.start_module(&name),
            Command::Stop(name) => self.stop_module(&name),
            Command::LogTail => {
                kprintln!("{}", format_log_tail_empty());
            }
            Command::Help(_) => self.print_help(),
            Command::Catalog => self.print_catalog(),
            Command::Install(name) => self.install_module(&name),
            Command::Remove(name) => self.remove_module(&name),
            Command::Setup => self.run_setup_wizard(),
            Command::Login(user) => self.login(&user),
            Command::Logout => self.logout(),
            Command::Whoami => self.whoami(),
            Command::Users => self.list_users(),
            Command::UserAdd(user) => self.user_add(&user),
            Command::Pwd => self.print_pwd(),
            Command::Ls(path) => self.list_dir(path.as_deref()),
            Command::Cd(path) => self.change_dir(&path),
            Command::Mkdir(path) => self.make_dir(&path),
            Command::Touch(path) => self.touch_file(&path),
            Command::Cat(path) => self.cat_file(&path),
            Command::Edit(path) => self.edit_file(&path),
            Command::Cp { src, dst, recursive } => self.copy_path(&src, &dst, recursive),
            Command::Mv { src, dst } => self.move_path(&src, &dst),
            Command::MkdirP(path) => self.make_dir_p(&path),
            Command::Write { path, contents } => self.write_file(&path, &contents),
            Command::Rm(path) => self.remove_path(&path),
            Command::RmRecursive(path) => self.remove_path_recursive(&path),
            Command::Slots => self.print_slots(),
            Command::Plug { slot, module } => self.plug_slot(&slot, &module),
            Command::Unplug(slot) => self.unplug_slot(&slot),
            Command::Sysinfo => self.print_sysinfo(),
            Command::Unknown(_) => {
                if !raw.trim().is_empty() {
                    kprintln!("{}", format_unknown_command(raw.trim()));
                    self.print_help();
                }
            }
        }
    }

    fn print_help(&self) {
        kprintln!("{}", format_help());
    }

    fn print_running(&self) {
        let rows = self
            .modules
            .iter()
            .filter(|module| module.running)
            .map(|module| ProcessRow {
                pid: None,
                name: module.name.clone(),
                state: "running".to_string(),
            })
            .collect::<Vec<ProcessRow>>();
        kprintln!("{}", format_processes(&rows));
    }

    fn print_modules(&self) {
        let rows = self
            .modules
            .iter()
            .map(|module| ModuleRow {
                name: module.name.clone(),
                state: if module.running {
                    "running".to_string()
                } else {
                    "stopped".to_string()
                },
                provides: module
                    .manifest
                    .as_ref()
                    .map(|manifest| manifest.provides.clone())
                    .unwrap_or_default(),
            })
            .collect::<Vec<ModuleRow>>();
        kprintln!("{}", format_modules(&rows));
    }

    fn print_catalog(&self) {
        let rows = self
            .catalog
            .iter()
            .map(|module| ModuleRow {
                name: module.name.clone(),
                state: "available".to_string(),
                provides: module.manifest.provides.clone(),
            })
            .collect::<Vec<ModuleRow>>();
        kprintln!("{}", format_catalog(&rows));
    }

    fn start_module(&mut self, name: &str) {
        let Some(module) = self.modules.iter_mut().find(|m| m.name == name) else {
            kprintln!("module not found: {}", name);
            return;
        };
        if module.running {
            kprintln!("module already running: {}", name);
            return;
        }
        module.running = true;
        if let Some(manifest) = &module.manifest {
            self.board.mark_running(&module.name, &manifest.slots);
        }
        kprintln!("module started: {}", name);
    }

    fn stop_module(&mut self, name: &str) {
        if name == "init" {
            kprintln!("init cannot be stopped");
            return;
        }
        let Some(module) = self.modules.iter_mut().find(|m| m.name == name) else {
            kprintln!("module not found: {}", name);
            return;
        };
        if !module.running {
            kprintln!("module already stopped: {}", name);
            return;
        }
        module.running = false;
        if let Some(manifest) = &module.manifest {
            detach_module_slots(&mut self.board, &module.name, &manifest.slots);
        }
        kprintln!("module stopped: {}", name);
    }

    fn install_module(&mut self, name: &str) {
        if self.modules.iter().any(|module| module.name == name) {
            kprintln!("module already installed: {}", name);
            return;
        }
        let Some(index) = self.catalog.iter().position(|module| module.name == name) else {
            kprintln!("module not found in catalog: {}", name);
            return;
        };
        let entry = self.catalog.remove(index);
        self.modules.push(ModuleEntry {
            name: entry.name.clone(),
            manifest: Some(entry.manifest),
            running: false,
        });
        kprintln!("module installed: {}", name);
    }

    fn remove_module(&mut self, name: &str) {
        if matches!(name, "init" | "console-service" | "tui-shell") {
            kprintln!("module cannot be removed: {}", name);
            return;
        }
        let Some(index) = self.modules.iter().position(|module| module.name == name) else {
            kprintln!("module not installed: {}", name);
            return;
        };
        if self.modules[index].running {
            kprintln!("module is running, stop it first: {}", name);
            return;
        }
        let entry = self.modules.remove(index);
        if let Some(manifest) = &entry.manifest {
            detach_module_slots(&mut self.board, &entry.name, &manifest.slots);
        }
        if let Some(manifest) = entry.manifest {
            self.catalog.push(CatalogEntry {
                name: entry.name.clone(),
                manifest,
            });
        }
        kprintln!("module removed: {}", name);
    }

    fn ensure_setup(&mut self) {
        if self.is_setup_complete() {
            return;
        }
        kprintln!("First boot detected. Starting setup wizard.");
        self.run_setup_wizard();
    }

    fn ensure_base_profile(&mut self) {
        let mut base_modules = vec![
            "fs-service",
            "user-service",
            "session-service",
            "settings-service",
            "sysinfo-service",
            "file-manager",
            "net-service",
            "setup-wizard",
        ];

        if let Some(editor) = self.preferred_editor() {
            base_modules.push(editor);
        }

        let mut changed = false;
        for name in base_modules {
            if !self.modules.iter().any(|module| module.name == name) {
                if self.catalog.iter().any(|entry| entry.name == name) {
                    self.install_module(name);
                    changed = true;
                }
            }
            if let Some(module) = self.modules.iter().find(|module| module.name == name) {
                if !module.running {
                    self.start_module(name);
                    changed = true;
                }
            }
        }

        if changed {
            kprintln!("base profile ready");
        }
    }

    fn preferred_editor(&self) -> Option<&'static str> {
        if self.module_available("vim-piece") {
            return Some("vim-piece");
        }
        if self.module_available("text-editor") {
            return Some("text-editor");
        }
        None
    }

    fn module_available(&self, name: &str) -> bool {
        self.modules.iter().any(|module| module.name == name)
            || self.catalog.iter().any(|entry| entry.name == name)
    }

    fn is_setup_complete(&self) -> bool {
        if self.users.list_users().is_empty() {
            return false;
        }
        self.fs.read_file("/etc/ruzzle.conf").is_ok()
    }

    fn run_setup_wizard(&mut self) {
        let username = prompt_line("Create admin user").unwrap_or_else(|| "root".to_string());
        let hostname = prompt_with_default("Hostname", self.settings.hostname());
        let locale = prompt_with_default("Locale", self.settings.locale());
        let timezone = prompt_with_default("Timezone", self.settings.timezone());
        let keyboard = prompt_with_default("Keyboard layout", self.settings.keyboard());

        let plan = SetupPlan::new(&username, true, &hostname, &locale, &timezone, &keyboard);
        match run_first_boot(&mut self.fs, &mut self.users, &mut self.settings, &plan) {
            Ok(report) => {
                kprintln!("setup complete. created {} directories.", report.created_dirs.len());
                let _ = self.session.login(&self.users, &report.user);
                self.file_manager = FileManager::new();
                let home = default_home_dir(&report.user);
                let _ = self.file_manager.cd(&self.fs, &home);
            }
            Err(err) => {
                kprintln!("setup failed: {}", format_setup_error(&err));
            }
        }
    }

    fn login(&mut self, user: &str) {
        match self.session.login(&self.users, user) {
            Ok(()) => {
                let home = default_home_dir(user);
                let _ = self.file_manager.cd(&self.fs, &home);
                kprintln!("logged in as {}", user);
            }
            Err(_) => {
                kprintln!("login failed for {}", user);
            }
        }
    }

    fn logout(&mut self) {
        match self.session.logout() {
            Ok(()) => kprintln!("logged out"),
            Err(_) => kprintln!("no active session"),
        }
    }

    fn whoami(&self) {
        match self.session.active_user() {
            Some(user) => kprintln!("{}", user),
            None => kprintln!("<none>"),
        }
    }

    fn list_users(&self) {
        let users = self.users.list_users();
        if users.is_empty() {
            kprintln!("users:\n  <none>");
            return;
        }
        kprintln!("users:");
        for user in users {
            let role = if user.is_admin { "admin" } else { "user" };
            kprintln!("  {} ({}) home={}", user.name, role, user.home_dir);
        }
    }

    fn user_add(&mut self, name: &str) {
        let Some(active) = self.session.active_user() else {
            kprintln!("login required");
            return;
        };
        let Some(user) = self.users.get_user(active) else {
            kprintln!("login required");
            return;
        };
        if !user.is_admin {
            kprintln!("admin privilege required");
            return;
        }
        if let Err(err) = self.users.add_user(name, false) {
            kprintln!("user add failed: {:?}", err);
            return;
        }
        let home = default_home_dir(name);
        if let Err(err) = create_home_dirs(&mut self.fs, &home) {
            kprintln!("user created but home setup failed: {:?}", err);
        } else {
            kprintln!("user added: {}", name);
        }
    }

    fn print_pwd(&self) {
        kprintln!("{}", self.file_manager.pwd());
    }

    fn list_dir(&self, path: Option<&str>) {
        if self.require_login().is_none() {
            return;
        }
        let result = if let Some(path) = path {
            self.file_manager.ls_path(&self.fs, path)
        } else {
            self.file_manager.ls(&self.fs)
        };
        match result {
            Ok(list) => {
                if list.is_empty() {
                    kprintln!("<empty>");
                } else {
                    for entry in list {
                        kprintln!("{}", entry);
                    }
                }
            }
            Err(err) => kprintln!("ls error: {:?}", err),
        }
    }

    fn change_dir(&mut self, path: &str) {
        if self.require_login().is_none() {
            return;
        }
        match self.file_manager.cd(&self.fs, path) {
            Ok(()) => kprintln!("cwd={}", self.file_manager.pwd()),
            Err(err) => kprintln!("cd error: {:?}", err),
        }
    }

    fn make_dir(&mut self, path: &str) {
        if self.require_login().is_none() {
            return;
        }
        match self.file_manager.mkdir(&mut self.fs, path) {
            Ok(()) => kprintln!("dir created"),
            Err(err) => kprintln!("mkdir error: {:?}", err),
        }
    }

    fn make_dir_p(&mut self, path: &str) {
        let resolved = match self.file_manager.resolve(path) {
            Ok(path) => path,
            Err(err) => {
                kprintln!("mkdir -p error: {:?}", err);
                return;
            }
        };
        if resolved == "/" {
            kprintln!("dir exists");
            return;
        }
        let mut current = String::new();
        for segment in resolved.split('/').filter(|s| !s.is_empty()) {
            if current.is_empty() {
                current = format!("/{}", segment);
            } else {
                current = format!("{}/{}", current, segment);
            }
            match self.fs.mkdir(&current) {
                Ok(()) | Err(FsError::AlreadyExists) => {}
                Err(err) => {
                    kprintln!("mkdir -p error: {:?}", err);
                    return;
                }
            }
        }
        kprintln!("dir created");
    }

    fn touch_file(&mut self, path: &str) {
        if self.require_login().is_none() {
            return;
        }
        match self.file_manager.write(&mut self.fs, path, "") {
            Ok(()) => kprintln!("file ready"),
            Err(err) => kprintln!("touch error: {:?}", err),
        }
    }

    fn cat_file(&self, path: &str) {
        if self.require_login().is_none() {
            return;
        }
        match self.file_manager.cat(&self.fs, path) {
            Ok(text) => kprintln!("{}", text),
            Err(err) => kprintln!("cat error: {:?}", err),
        }
    }

    fn edit_file(&mut self, path: &str) {
        let Some(provider) = self.board.provider_for("ruzzle.slot.editor") else {
            kprintln!("editor slot is empty. plug a piece into ruzzle.slot.editor first.");
            return;
        };

        let contents = match self.file_manager.cat(&self.fs, path) {
            Ok(text) => text,
            Err(FsError::NotFound) => String::new(),
            Err(err) => {
                kprintln!("edit error: {:?}", err);
                return;
            }
        };

        let mut buffer = TextBuffer::from_text(&contents);
        kprintln!("editor: {} (provider={})", path, provider);
        print_editor_help();

        loop {
            kprint!("edit> ");
            let input = read_line();
            match parse_editor_command(&input) {
                EditorCommand::Append(text) => {
                    let index = buffer.line_count();
                    if let Err(err) = buffer.insert_line(index, &text) {
                        kprintln!("append error: {:?}", err);
                    }
                }
                EditorCommand::Insert { index, text } => {
                    if let Err(err) = buffer.insert_line(index, &text) {
                        kprintln!("insert error: {:?}", err);
                    }
                }
                EditorCommand::Replace { index, text } => {
                    if let Err(err) = buffer.replace_line(index, &text) {
                        kprintln!("replace error: {:?}", err);
                    }
                }
                EditorCommand::Delete(index) => {
                    if let Err(err) = buffer.remove_line(index) {
                        kprintln!("delete error: {:?}", err);
                    }
                }
                EditorCommand::Print => {
                    print_editor_buffer(&buffer);
                }
                EditorCommand::Save => {
                    if save_editor_buffer(&mut self.file_manager, &mut self.fs, path, &buffer) {
                        kprintln!("saved");
                    }
                }
                EditorCommand::SaveQuit => {
                    if save_editor_buffer(&mut self.file_manager, &mut self.fs, path, &buffer) {
                        kprintln!("saved");
                        break;
                    }
                }
                EditorCommand::Quit => {
                    kprintln!("editor closed");
                    break;
                }
                EditorCommand::Help => {
                    print_editor_help();
                }
                EditorCommand::Unknown => {
                    kprintln!("editor: unknown command");
                    print_editor_help();
                }
            }
        }
    }

    fn write_file(&mut self, path: &str, contents: &str) {
        if self.require_login().is_none() {
            return;
        }
        match self.file_manager.write(&mut self.fs, path, contents) {
            Ok(()) => kprintln!("write ok"),
            Err(err) => kprintln!("write error: {:?}", err),
        }
    }

    fn remove_path(&mut self, path: &str) {
        if self.require_login().is_none() {
            return;
        }
        match self.file_manager.rm(&mut self.fs, path) {
            Ok(()) => kprintln!("removed"),
            Err(err) => kprintln!("rm error: {:?}", err),
        }
    }

    fn remove_path_recursive(&mut self, path: &str) {
        let resolved = match self.file_manager.resolve(path) {
            Ok(path) => path,
            Err(err) => {
                kprintln!("rm -r error: {:?}", err);
                return;
            }
        };
        match remove_recursive(&mut self.fs, &resolved) {
            Ok(()) => kprintln!("removed"),
            Err(err) => kprintln!("rm -r error: {:?}", err),
        }
    }

    fn copy_path(&mut self, src: &str, dst: &str, recursive: bool) {
        let src_path = match self.file_manager.resolve(src) {
            Ok(path) => path,
            Err(err) => {
                kprintln!("cp error: {:?}", err);
                return;
            }
        };
        let dst_path = match self.file_manager.resolve(dst) {
            Ok(path) => path,
            Err(err) => {
                kprintln!("cp error: {:?}", err);
                return;
            }
        };
        match copy_recursive(&mut self.fs, &src_path, &dst_path, recursive) {
            Ok(()) => kprintln!("copied"),
            Err(err) => kprintln!("cp error: {:?}", err),
        }
    }

    fn move_path(&mut self, src: &str, dst: &str) {
        let src_path = match self.file_manager.resolve(src) {
            Ok(path) => path,
            Err(err) => {
                kprintln!("mv error: {:?}", err);
                return;
            }
        };
        let dst_path = match self.file_manager.resolve(dst) {
            Ok(path) => path,
            Err(err) => {
                kprintln!("mv error: {:?}", err);
                return;
            }
        };
        match copy_recursive(&mut self.fs, &src_path, &dst_path, true) {
            Ok(()) => match remove_recursive(&mut self.fs, &src_path) {
                Ok(()) => kprintln!("moved"),
                Err(err) => kprintln!("mv cleanup error: {:?}", err),
            },
            Err(err) => kprintln!("mv error: {:?}", err),
        }
    }

    fn print_slots(&self) {
        let rows = self
            .board
            .list()
            .into_iter()
            .map(|slot| SlotRow {
                name: slot.name,
                required: slot.required,
                provider: slot.provider,
            })
            .collect::<Vec<SlotRow>>();
        kprintln!("{}", format_slots(&rows));
    }

    fn plug_slot(&mut self, slot: &str, module: &str) {
        let Some(entry) = self.modules.iter().find(|entry| entry.name == module) else {
            kprintln!("module not found: {}", module);
            return;
        };
        let Some(manifest) = &entry.manifest else {
            kprintln!("module has no manifest: {}", module);
            return;
        };
        match self.board.plug(slot, module, &manifest.slots) {
            Ok(()) => kprintln!("plugged {} -> {}", slot, module),
            Err(err) => kprintln!("plug failed: {:?}", err),
        }
    }

    fn unplug_slot(&mut self, slot: &str) {
        match self.board.unplug(slot) {
            Ok(Some(provider)) => kprintln!("unplugged {} from {}", slot, provider),
            Ok(None) => kprintln!("slot already empty: {}", slot),
            Err(BoardError::SlotNotFound) => kprintln!("slot not found: {}", slot),
            Err(err) => kprintln!("unplug failed: {:?}", err),
        }
    }

    fn print_sysinfo(&self) {
        let info = build_system_info(&self.settings, &self.session, &self.board);
        kprintln!("{}", format_system_info(&info));
    }

    fn require_login(&self) -> Option<&str> {
        if let Some(user) = self.session.active_user() {
            Some(user)
        } else {
            kprintln!("login required");
            None
        }
    }
}

fn command_requires_login(command: &Command) -> bool {
    !matches!(
        command,
        Command::Help(_)
            | Command::Login(_)
            | Command::Logout
            | Command::Setup
            | Command::Whoami
            | Command::Unknown(_)
    )
}

fn join_path(base: &str, child: &str) -> String {
    if base == "/" {
        format!("/{}", child)
    } else {
        format!("{}/{}", base, child)
    }
}

fn remove_recursive(fs: &mut FileSystem, path: &str) -> Result<(), FsError> {
    match fs.list_dir(path) {
        Ok(entries) => {
            for entry in entries {
                let child = join_path(path, &entry);
                remove_recursive(fs, &child)?;
            }
        }
        Err(FsError::NotDir) => {
            return fs.remove(path);
        }
        Err(err) => return Err(err),
    }
    fs.remove(path)
}

fn copy_recursive(
    fs: &mut FileSystem,
    src: &str,
    dst: &str,
    recursive: bool,
) -> Result<(), FsError> {
    match fs.read_file(src) {
        Ok(data) => return fs.write_file(dst, &data),
        Err(FsError::IsDir) => {}
        Err(err) => return Err(err),
    }

    if !recursive {
        return Err(FsError::IsDir);
    }

    match fs.list_dir(dst) {
        Ok(_) => {}
        Err(FsError::NotFound) => {
            fs.mkdir(dst)?;
        }
        Err(FsError::NotDir) => {
            return Err(FsError::NotDir);
        }
        Err(err) => return Err(err),
    }

    let entries = fs.list_dir(src)?;
    for entry in entries {
        let src_child = join_path(src, &entry);
        let dst_child = join_path(dst, &entry);
        copy_recursive(fs, &src_child, &dst_child, recursive)?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum EditorCommand {
    Append(String),
    Insert { index: usize, text: String },
    Replace { index: usize, text: String },
    Delete(usize),
    Print,
    Save,
    Quit,
    SaveQuit,
    Help,
    Unknown,
}

fn parse_editor_command(input: &str) -> EditorCommand {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return EditorCommand::Unknown;
    }
    match trimmed {
        ":w" => return EditorCommand::Save,
        ":q" => return EditorCommand::Quit,
        ":wq" => return EditorCommand::SaveQuit,
        ":p" | "p" => return EditorCommand::Print,
        ":h" | ":help" | "help" => return EditorCommand::Help,
        _ => {}
    }

    let mut parts = trimmed.split_whitespace();
    let cmd = parts.next().unwrap_or("");
    match cmd {
        "a" => {
            let text = parts.collect::<Vec<&str>>().join(" ");
            if text.is_empty() {
                EditorCommand::Unknown
            } else {
                EditorCommand::Append(text)
            }
        }
        "i" => parse_editor_indexed(parts, EditorIndexedKind::Insert),
        "r" => parse_editor_indexed(parts, EditorIndexedKind::Replace),
        "d" => {
            let Some(index) = parse_editor_index(parts.next()) else {
                return EditorCommand::Unknown;
            };
            EditorCommand::Delete(index)
        }
        _ => EditorCommand::Unknown,
    }
}

enum EditorIndexedKind {
    Insert,
    Replace,
}

fn parse_editor_indexed<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    kind: EditorIndexedKind,
) -> EditorCommand {
    let Some(index) = parse_editor_index(parts.next()) else {
        return EditorCommand::Unknown;
    };
    let text = parts.collect::<Vec<&str>>().join(" ");
    if text.is_empty() {
        return EditorCommand::Unknown;
    }
    match kind {
        EditorIndexedKind::Insert => EditorCommand::Insert { index, text },
        EditorIndexedKind::Replace => EditorCommand::Replace { index, text },
    }
}

fn parse_editor_index(value: Option<&str>) -> Option<usize> {
    let raw = value?.parse::<usize>().ok()?;
    if raw == 0 {
        return None;
    }
    Some(raw - 1)
}

fn print_editor_buffer(buffer: &TextBuffer) {
    if buffer.line_count() == 0 {
        kprintln!("<empty>");
        return;
    }
    for (index, line) in buffer.lines().iter().enumerate() {
        kprintln!("{:>3} {}", index + 1, line);
    }
}

fn save_editor_buffer(
    file_manager: &mut FileManager,
    fs: &mut FileSystem,
    path: &str,
    buffer: &TextBuffer,
) -> bool {
    match file_manager.write(fs, path, &buffer.to_text()) {
        Ok(()) => true,
        Err(err) => {
            kprintln!("save error: {:?}", err);
            false
        }
    }
}

fn print_editor_help() {
    kprintln!("editor commands:");
    kprintln!("  :w               save");
    kprintln!("  :q               quit");
    kprintln!("  :wq              save and quit");
    kprintln!("  :p | p           print buffer");
    kprintln!("  a <text>         append line");
    kprintln!("  i <n> <text>     insert at line n");
    kprintln!("  r <n> <text>     replace line n");
    kprintln!("  d <n>            delete line n");
    kprintln!("  :h | help        show help");
}

fn build_modules(initramfs: Option<&[u8]>) -> (Vec<ModuleEntry>, Vec<CatalogEntry>) {
    let mut modules = Vec::new();
    let mut catalog = Vec::new();
    let Some(initramfs) = initramfs else {
        return (modules, catalog);
    };

    let Ok(entries) = parse_initramfs(initramfs) else {
        return (modules, catalog);
    };

    let mut manifests = Vec::new();
    for entry in &entries {
        if is_piece_bundle(&entry.name) {
            if let Ok(bundle) = parse_module_bundle(&entry.data) {
                catalog.push(CatalogEntry {
                    name: bundle.manifest.name.clone(),
                    manifest: bundle.manifest,
                });
            }
            continue;
        }
        if !entry.name.ends_with(".module.toml") {
            continue;
        }
        if let Ok(text) = core::str::from_utf8(&entry.data) {
            if let Ok(manifest) = parse_module_manifest(text) {
                manifests.push(manifest);
            }
        }
    }

    for manifest in manifests {
        modules.push(ModuleEntry {
            name: manifest.name.clone(),
            manifest: Some(manifest),
            running: false,
        });
    }

    for entry in &entries {
        if entry.name.ends_with(".module.toml") {
            continue;
        }
        if is_piece_bundle(&entry.name) {
            continue;
        }
        if modules.iter().any(|module| module.name == entry.name) {
            continue;
        }
        modules.push(ModuleEntry {
            name: entry.name.clone(),
            manifest: None,
            running: false,
        });
    }

    mark_running(&mut modules, &["init", "console-service", "tui-shell"]);
    catalog.retain(|entry| !modules.iter().any(|module| module.name == entry.name));
    (modules, catalog)
}

fn build_puzzle_board(modules: &[ModuleEntry]) -> PuzzleBoard {
    let mut board = PuzzleBoard::new(default_slots());
    for module in modules {
        if module.running {
            if let Some(manifest) = &module.manifest {
                board.mark_running(&module.name, &manifest.slots);
            }
        }
    }
    board
}

fn default_slots() -> Vec<PuzzleSlot> {
    vec![
        PuzzleSlot::new("ruzzle.slot.console", true),
        PuzzleSlot::new("ruzzle.slot.shell", true),
        PuzzleSlot::new("ruzzle.slot.fs", true),
        PuzzleSlot::new("ruzzle.slot.user", true),
        PuzzleSlot::new("ruzzle.slot.settings", true),
        PuzzleSlot::new("ruzzle.slot.session", true),
        PuzzleSlot::new("ruzzle.slot.setup", false),
        PuzzleSlot::new("ruzzle.slot.net", false),
        PuzzleSlot::new("ruzzle.slot.editor", false),
        PuzzleSlot::new("ruzzle.slot.filemgr", false),
        PuzzleSlot::new("ruzzle.slot.sysinfo", false),
    ]
}

fn detach_module_slots(board: &mut PuzzleBoard, module: &str, slots: &[String]) {
    for slot in slots {
        if let Ok(Some(provider)) = board.unplug(slot) {
            if provider != module {
                let _ = board.plug(slot, &provider, &[slot.to_string()]);
            }
        }
    }
}

fn is_piece_bundle(name: &str) -> bool {
    name.ends_with(".rpiece")
}

fn create_home_dirs(fs: &mut FileSystem, home: &str) -> Result<(), FsError> {
    match fs.mkdir(home) {
        Ok(()) | Err(FsError::AlreadyExists) => {}
        Err(err) => return Err(err),
    }
    for suffix in ["docs", "bin", ".config", "downloads"].iter() {
        let path = format!("{}/{}", home, suffix);
        match fs.mkdir(&path) {
            Ok(()) | Err(FsError::AlreadyExists) => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

fn format_setup_error(err: &SetupError) -> &'static str {
    match err {
        SetupError::InvalidUser => "invalid user name",
        SetupError::User(_) => "user error",
        SetupError::Fs(_) => "filesystem error",
        SetupError::Settings(_) => "settings error",
    }
}

fn mark_running(modules: &mut [ModuleEntry], names: &[&str]) {
    for &name in names {
        if let Some(module) = modules.iter_mut().find(|module| module.name == name) {
            module.running = true;
        }
    }
}

fn read_line() -> String {
    let mut line = String::new();
    loop {
        if !console::has_input() {
            core::hint::spin_loop();
            continue;
        }
        let byte = console::read_byte();
        match byte {
            b'\r' | b'\n' => {
                kprintln!();
                break;
            }
            0x08 | 0x7f => {
                if !line.is_empty() {
                    line.pop();
                    kprint!("\x08 \x08");
                }
            }
            _ => {
                if byte.is_ascii_graphic() || byte == b' ' {
                    line.push(byte as char);
                    kprint!("{}", byte as char);
                }
            }
        }
    }
    line
}

fn prompt_line(label: &str) -> Option<String> {
    kprint!("{}: ", label);
    let input = read_line();
    let trimmed = input.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn prompt_with_default(label: &str, default: &str) -> String {
    kprint!("{} [{}]: ", label, default);
    let input = read_line();
    let trimmed = input.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}
