use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use kernel_core::{parse_initramfs, parse_module_bundle, parse_module_manifest, ModuleManifest};
use user_file_manager::FileManager;
use user_fs_service::{FileSystem, FsError};
use user_net_service::NetManager;
use user_puzzle_board::{BoardError, PuzzleBoard, PuzzleSlot};
use user_session_service::SessionManager;
use user_settings_service::SystemSettings;
use user_setup_wizard::{run_first_boot, SetupPlan, SetupError};
use user_sysinfo_service::{build_system_info, format_system_info, SystemMetrics};
use user_text_editor::TextBuffer;
use user_tui_shell::{
    format_catalog, format_graph, format_help, format_log_tail_empty, format_modules,
    format_processes, format_slots, format_unknown_command, parse_command, Command, GraphRow,
    ModuleRow, ProcessRow, SlotRow,
};
use user_user_service::{default_home_dir, UserManager};

use crate::{console, kprint, kprintln, smp};

#[derive(Debug, Clone)]
struct ModuleEntry {
    name: String,
    manifest: Option<ModuleManifest>,
    running: bool,
    verified: bool,
}

#[derive(Debug, Clone)]
struct CatalogEntry {
    name: String,
    manifest: ModuleManifest,
    verified: bool,
}

#[derive(Debug, Clone)]
struct MountEntry {
    source: String,
    target: String,
    fstype: String,
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
    initramfs: Option<Vec<u8>>,
    fs: FileSystem,
    file_manager: FileManager,
    net: NetManager,
    mounts: Vec<MountEntry>,
    users: UserManager,
    session: SessionManager,
    settings: SystemSettings,
    board: PuzzleBoard,
    login_tip_shown: bool,
}

impl ShellState {
    fn new(initramfs: Option<&[u8]>) -> Self {
        let initramfs_data = initramfs.map(|data| data.to_vec());
        let (modules, catalog) = build_modules(initramfs);
        let fs = FileSystem::new();
        let file_manager = FileManager::new();
        let net = NetManager::new();
        let mounts = default_mounts();
        let users = UserManager::new();
        let session = SessionManager::new();
        let settings = SystemSettings::new_defaults();
        let board = build_puzzle_board(&modules);
        let mut state = Self {
            modules,
            catalog,
            initramfs: initramfs_data,
            fs,
            file_manager,
            net,
            mounts,
            users,
            session,
            settings,
            board,
            login_tip_shown: false,
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
            Command::Ps { tree } => self.print_running(tree),
            Command::Lsmod => self.print_modules(),
            Command::Start(name) => self.start_module(&name),
            Command::Stop(name) => self.stop_module(&name),
            Command::LogTail => {
                kprintln!("{}", format_log_tail_empty());
            }
            Command::Help(topic) => self.print_help(topic.as_deref()),
            Command::Catalog {
                slot,
                verified_only,
            } => self.print_catalog(slot.as_deref(), verified_only),
            Command::PieceCheck(name) => self.piece_check(&name),
            Command::Ip(args) => self.run_ip(args.as_deref()),
            Command::Route(args) => self.run_route(args.as_deref()),
            Command::Mount(args) => self.run_mount(args.as_deref()),
            Command::Df(path) => self.print_df(path.as_deref()),
            Command::Du(path) => self.print_du(&path),
            Command::MarketScan => self.market_scan(),
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
            Command::Plug {
                slot,
                module,
                dry_run,
                swap,
            } => self.plug_slot(&slot, &module, dry_run, swap),
            Command::Unplug(slot) => self.unplug_slot(&slot),
            Command::Graph => self.print_graph(),
            Command::Sysinfo => self.print_sysinfo(),
            Command::Unknown(_) => {
                if !raw.trim().is_empty() {
                    kprintln!("{}", format_unknown_command(raw.trim()));
                    self.print_help();
                }
            }
        }
    }

    fn print_help(&self, topic: Option<&str>) {
        kprintln!("{}", format_help(topic));
    }

    fn print_running(&self, tree: bool) {
        let running = self
            .modules
            .iter()
            .filter(|module| module.running)
            .collect::<Vec<&ModuleEntry>>();
        if !tree {
            let rows = running
                .iter()
                .map(|module| ProcessRow {
                    pid: None,
                    name: module.name.clone(),
                    state: "running".to_string(),
                })
                .collect::<Vec<ProcessRow>>();
            kprintln!("{}", format_processes(&rows));
            return;
        }

        kprintln!("processes (tree):");
        if running.is_empty() {
            kprintln!("  <none>");
            return;
        }

        let has_init = running.iter().any(|module| module.name == "init");
        let root = if has_init { "init" } else { "<root>" };
        kprintln!("  {} [running]", root);

        let mut children = running
            .iter()
            .filter(|module| module.name != "init")
            .collect::<Vec<&&ModuleEntry>>();
        children.sort_by(|a, b| a.name.cmp(&b.name));
        for (index, module) in children.iter().enumerate() {
            let branch = if index + 1 == children.len() { "`-" } else { "|-" };
            kprintln!("  {} {} [running]", branch, module.name);
        }
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

    fn print_catalog(&self, slot: Option<&str>, verified_only: bool) {
        let slot_filter = match slot {
            Some(value) => match normalize_slot_filter(value) {
                Ok(normalized) => Some(normalized),
                Err(_) => {
                    kprintln!("invalid slot filter: {}", value);
                    return;
                }
            },
            None => None,
        };
        let rows = self
            .catalog
            .iter()
            .filter(|module| {
                if verified_only && !module.verified {
                    return false;
                }
                if let Some(filter) = &slot_filter {
                    return module.manifest.slots.iter().any(|slot| slot == filter);
                }
                true
            })
            .map(|module| ModuleRow {
                name: module.name.clone(),
                state: if module.verified {
                    "verified".to_string()
                } else {
                    "unsigned".to_string()
                },
                provides: module.manifest.provides.clone(),
            })
            .collect::<Vec<ModuleRow>>();
        kprintln!("{}", format_catalog(&rows));
    }

    fn piece_check(&self, name: &str) {
        let module_entry = self.modules.iter().find(|module| module.name == name);
        let catalog_entry = self.catalog.iter().find(|entry| entry.name == name);

        let (manifest, location, run_state, verified) = if let Some(module) = module_entry {
            (
                module.manifest.as_ref(),
                "installed",
                if module.running { "running" } else { "stopped" },
                module.verified,
            )
        } else if let Some(entry) = catalog_entry {
            (
                Some(&entry.manifest),
                "catalog",
                "available",
                entry.verified,
            )
        } else {
            kprintln!("piece not found: {}", name);
            return;
        };

        let Some(manifest) = manifest else {
            kprintln!("piece has no manifest: {}", name);
            return;
        };

        let signature = if verified { "verified" } else { "unsigned" };
        kprintln!("piece check: {}", name);
        kprintln!("  location: {} ({})", location, run_state);
        kprintln!("  signature: {}", signature);

        let board_slots = self.board.list();
        let mut warnings = Vec::new();

        kprintln!("  dependencies:");
        if manifest.depends.is_empty() {
            kprintln!("    <none>");
        } else {
            for dep in &manifest.depends {
                let status = if let Some(dep_module) =
                    self.modules.iter().find(|module| module.name == *dep)
                {
                    if dep_module.running {
                        "running"
                    } else {
                        warnings.push(format!("dependency stopped: {}", dep));
                        "installed"
                    }
                } else if let Some(dep_entry) =
                    self.catalog.iter().find(|entry| entry.name == *dep)
                {
                    if dep_entry.verified {
                        warnings.push(format!("dependency not installed: {}", dep));
                        "catalog"
                    } else {
                        warnings.push(format!("dependency unsigned: {}", dep));
                        "unsigned"
                    }
                } else {
                    warnings.push(format!("dependency missing: {}", dep));
                    "missing"
                };
                kprintln!("    {} [{}]", dep, status);
            }
        }

        kprintln!("  slots:");
        if manifest.slots.is_empty() {
            kprintln!("    <none>");
        } else {
            for slot in &manifest.slots {
                let slot_entry = board_slots.iter().find(|entry| entry.name == *slot);
                let status = match slot_entry {
                    Some(entry) => match entry.provider.as_deref() {
                        Some(provider) if provider == name => "active".to_string(),
                        Some(provider) => {
                            warnings.push(format!(
                                "slot occupied: {} -> {}",
                                slot, provider
                            ));
                            format!("occupied by {}", provider)
                        }
                        None => {
                            warnings.push(format!("slot empty: {}", slot));
                            "empty".to_string()
                        }
                    },
                    None => {
                        warnings.push(format!("slot missing: {}", slot));
                        "missing".to_string()
                    }
                };
                kprintln!("    {} [{}]", slot, status);
            }
        }

        if !warnings.is_empty() {
            kprintln!("warnings:");
            for warning in &warnings {
                kprintln!("  - {}", warning);
            }
        }

        let mut rows = Vec::new();
        rows.push(GraphRow {
            name: name.to_string(),
            state: run_state.to_string(),
            depends: manifest.depends.clone(),
        });
        let mut deps_sorted = manifest.depends.clone();
        deps_sorted.sort();
        deps_sorted.dedup();
        for dep in deps_sorted {
            let (state, depends) = if let Some(dep_module) =
                self.modules.iter().find(|module| module.name == dep)
            {
                (
                    if dep_module.running {
                        "running".to_string()
                    } else {
                        "installed".to_string()
                    },
                    dep_module
                        .manifest
                        .as_ref()
                        .map(|manifest| manifest.depends.clone())
                        .unwrap_or_default(),
                )
            } else if let Some(dep_entry) = self.catalog.iter().find(|entry| entry.name == dep) {
                (
                    if dep_entry.verified {
                        "catalog".to_string()
                    } else {
                        "unsigned".to_string()
                    },
                    dep_entry.manifest.depends.clone(),
                )
            } else {
                ("missing".to_string(), Vec::new())
            };
            rows.push(GraphRow {
                name: dep,
                state,
                depends,
            });
        }

        kprintln!("{}", format_graph(&rows));
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
        if !self.catalog[index].verified {
            kprintln!("module not verified: {}", name);
            return;
        }
        let entry = self.catalog.remove(index);
        let manifest = entry.manifest.clone();
        self.modules.push(ModuleEntry {
            name: entry.name.clone(),
            manifest: Some(entry.manifest),
            running: false,
            verified: entry.verified,
        });
        kprintln!("module installed: {}", name);
        self.print_manifest_summary(&manifest);
    }

    fn print_manifest_summary(&self, manifest: &ModuleManifest) {
        kprintln!("  version: {}", manifest.version);
        kprintln!("  provides: {}", join_list(&manifest.provides));
        kprintln!("  slots: {}", join_list(&manifest.slots));
        kprintln!("  requires: {}", join_list(&manifest.requires_caps));
        kprintln!("  depends: {}", join_list(&manifest.depends));
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
                verified: entry.verified,
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
            "net-manager",
            "input-service",
            "device-manager",
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
                self.show_login_tips(&report.user);
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
                self.show_login_tips(user);
            }
            Err(_) => {
                kprintln!("login failed for {}", user);
            }
        }
    }

    fn show_login_tips(&mut self, user: &str) {
        if self.login_tip_shown {
            return;
        }
        self.login_tip_shown = true;
        kprintln!("welcome, {}!", user);
        kprintln!("tips:");
        kprintln!("  slots            # view puzzle slots");
        kprintln!("  graph            # dependency graph");
        kprintln!("  ps --tree        # show process tree");
        kprintln!("  catalog --verified");
        kprintln!("  install <piece>");
        kprintln!("  help slot | help market");
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
        let Some(provider) = self.board.provider_for("ruzzle.slot.editor@1") else {
            kprintln!("editor slot is empty. plug a piece into ruzzle.slot.editor@1 first.");
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

    fn run_ip(&mut self, args: Option<&str>) {
        let Some(args) = args else {
            self.print_interfaces();
            return;
        };
        let mut parts = args.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let rest = parts.collect::<Vec<&str>>();
        match cmd {
            "add" => {
                if rest.len() != 1 {
                    kprintln!("ip add <iface>");
                    return;
                }
                match self.net.add_interface(rest[0]) {
                    Ok(()) => kprintln!("interface added: {}", rest[0]),
                    Err(err) => kprintln!("ip error: {:?}", err),
                }
            }
            "del" => {
                if rest.len() != 1 {
                    kprintln!("ip del <iface>");
                    return;
                }
                match self.net.remove_interface(rest[0]) {
                    Ok(()) => kprintln!("interface removed: {}", rest[0]),
                    Err(err) => kprintln!("ip error: {:?}", err),
                }
            }
            "up" => {
                if rest.len() != 1 {
                    kprintln!("ip up <iface>");
                    return;
                }
                match self.net.set_up(rest[0], true) {
                    Ok(()) => kprintln!("interface up: {}", rest[0]),
                    Err(err) => kprintln!("ip error: {:?}", err),
                }
            }
            "down" => {
                if rest.len() != 1 {
                    kprintln!("ip down <iface>");
                    return;
                }
                match self.net.set_up(rest[0], false) {
                    Ok(()) => kprintln!("interface down: {}", rest[0]),
                    Err(err) => kprintln!("ip error: {:?}", err),
                }
            }
            "addr" => {
                if rest.len() != 2 {
                    kprintln!("ip addr <iface> <addr|none>");
                    return;
                }
                let addr = match rest[1] {
                    "none" | "-" => None,
                    value => Some(value),
                };
                match self.net.set_ipv4(rest[0], addr) {
                    Ok(()) => kprintln!("ip addr updated: {}", rest[0]),
                    Err(err) => kprintln!("ip error: {:?}", err),
                }
            }
            _ => {
                kprintln!("ip [add|del|up|down|addr]");
            }
        }
    }

    fn print_interfaces(&self) {
        let list = self.net.list();
        if list.is_empty() {
            kprintln!("interfaces:\n  <none>");
            return;
        }
        kprintln!("interfaces:");
        for iface in list {
            let state = if iface.up { "up" } else { "down" };
            let addr = iface.ipv4.as_deref().unwrap_or("-");
            kprintln!("  {} [{}] ipv4={}", iface.name, state, addr);
        }
    }

    fn run_route(&mut self, args: Option<&str>) {
        let Some(args) = args else {
            self.print_routes();
            return;
        };
        let mut parts = args.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let rest = parts.collect::<Vec<&str>>();
        match cmd {
            "add" => {
                if rest.len() != 2 {
                    kprintln!("route add <dest> <iface>");
                    return;
                }
                match self.net.add_route(rest[0], rest[1]) {
                    Ok(()) => kprintln!("route added: {} -> {}", rest[0], rest[1]),
                    Err(err) => kprintln!("route error: {:?}", err),
                }
            }
            "del" => {
                if rest.len() != 1 {
                    kprintln!("route del <dest>");
                    return;
                }
                match self.net.remove_route(rest[0]) {
                    Ok(()) => kprintln!("route removed: {}", rest[0]),
                    Err(err) => kprintln!("route error: {:?}", err),
                }
            }
            _ => kprintln!("route [add|del]"),
        }
    }

    fn print_routes(&self) {
        let routes = self.net.list_routes();
        if routes.is_empty() {
            kprintln!("routes:\n  <none>");
            return;
        }
        kprintln!("routes:");
        for route in routes {
            kprintln!("  {} -> {}", route.destination, route.iface);
        }
    }

    fn run_mount(&mut self, args: Option<&str>) {
        let Some(args) = args else {
            self.print_mounts();
            return;
        };
        let parts = args.split_whitespace().collect::<Vec<&str>>();
        if parts.len() < 2 || parts.len() > 3 {
            kprintln!("mount <source> <target> [type]");
            return;
        }
        let source = parts[0];
        let target = match self.file_manager.resolve(parts[1]) {
            Ok(path) => path,
            Err(err) => {
                kprintln!("mount error: {:?}", err);
                return;
            }
        };
        if self.fs.list_dir(&target).is_err() {
            kprintln!("mount error: target not a directory");
            return;
        }
        if self.mounts.iter().any(|entry| entry.target == target) {
            kprintln!("mount error: target already mounted");
            return;
        }
        let fstype = if parts.len() == 3 { parts[2] } else { "memfs" };
        self.mounts.push(MountEntry {
            source: source.to_string(),
            target: target.clone(),
            fstype: fstype.to_string(),
        });
        kprintln!("mounted {} on {} ({})", source, target, fstype);
    }

    fn print_mounts(&self) {
        if self.mounts.is_empty() {
            kprintln!("mounts:\n  <none>");
            return;
        }
        kprintln!("mounts:");
        for entry in &self.mounts {
            kprintln!(
                "  {} {} ({})",
                entry.source, entry.target, entry.fstype
            );
        }
    }

    fn print_df(&self, path: Option<&str>) {
        let target = path.unwrap_or(self.file_manager.pwd());
        let resolved = match self.file_manager.resolve(target) {
            Ok(path) => path,
            Err(err) => {
                kprintln!("df error: {:?}", err);
                return;
            }
        };
        match self.fs.stats_for(&resolved) {
            Ok(stats) => {
                kprintln!("df {}", resolved);
                kprintln!("  files: {}", stats.files);
                kprintln!("  dirs: {}", stats.dirs);
                kprintln!("  bytes: {}", stats.bytes);
            }
            Err(err) => kprintln!("df error: {:?}", err),
        }
    }

    fn print_du(&self, path: &str) {
        let resolved = match self.file_manager.resolve(path) {
            Ok(path) => path,
            Err(err) => {
                kprintln!("du error: {:?}", err);
                return;
            }
        };
        match self.fs.size_of(&resolved) {
            Ok(bytes) => kprintln!("du {} -> {} bytes", resolved, bytes),
            Err(err) => kprintln!("du error: {:?}", err),
        }
    }

    fn market_scan(&mut self) {
        let Some(initramfs) = self.initramfs.as_deref() else {
            kprintln!("market scan: no initramfs available");
            return;
        };
        let Ok(entries) = parse_initramfs(initramfs) else {
            kprintln!("market scan failed: unable to parse initramfs");
            return;
        };
        let mut catalog = Vec::new();
        for entry in &entries {
            if !is_piece_bundle(&entry.name) {
                continue;
            }
            if let Ok(bundle) = parse_module_bundle(&entry.data) {
                catalog.push(CatalogEntry {
                    name: bundle.manifest.name.clone(),
                    manifest: bundle.manifest,
                    verified: bundle.verified,
                });
            }
        }
        catalog.retain(|entry| !self.modules.iter().any(|module| module.name == entry.name));
        let count = catalog.len();
        self.catalog = catalog;
        kprintln!("market scan complete: {} entries", count);
    }

    fn plug_slot(&mut self, slot: &str, module: &str, dry_run: bool, swap: bool) {
        let Some(entry) = self.modules.iter().find(|entry| entry.name == module) else {
            kprintln!("module not found: {}", module);
            return;
        };
        let Some(manifest) = &entry.manifest else {
            kprintln!("module has no manifest: {}", module);
            return;
        };
        match self.board.can_plug(slot, &manifest.slots) {
            Ok(()) => {
                if dry_run {
                    kprintln!("dry-run ok: {} -> {}", slot, module);
                    return;
                }
                match self.board.plug(slot, module, &manifest.slots) {
                    Ok(()) => kprintln!("plugged {} -> {}", slot, module),
                    Err(err) => kprintln!("plug failed: {:?}", err),
                }
            }
            Err(BoardError::SlotAlreadyFilled) => {
                if !swap {
                    if dry_run {
                        kprintln!("dry-run failed: slot already filled");
                    } else {
                        kprintln!("plug failed: slot already filled");
                    }
                    return;
                }
                let Some(current) = self.board.provider_for(slot) else {
                    kprintln!("swap failed: cannot resolve provider");
                    return;
                };
                if current == module {
                    if dry_run {
                        kprintln!("dry-run ok: slot already filled by {}", module);
                    } else {
                        kprintln!("slot already filled by {}", module);
                    }
                    return;
                }
                if dry_run {
                    kprintln!("dry-run swap: {} -> {} (replace {})", slot, module, current);
                    return;
                }
                let Some(old_entry) = self.modules.iter().find(|entry| entry.name == current)
                else {
                    kprintln!("swap failed: provider not installed: {}", current);
                    return;
                };
                let Some(old_manifest) = &old_entry.manifest else {
                    kprintln!("swap failed: provider has no manifest: {}", current);
                    return;
                };
                match self.board.unplug(slot) {
                    Ok(_) => match self.board.plug(slot, module, &manifest.slots) {
                        Ok(()) => kprintln!("swapped {} -> {} (was {})", slot, module, current),
                        Err(err) => {
                            let rollback = self
                                .board
                                .plug(slot, current, &old_manifest.slots)
                                .is_ok();
                            if rollback {
                                kprintln!("swap failed: {:?} (rolled back)", err);
                            } else {
                                kprintln!("swap failed: {:?} (rollback failed)", err);
                            }
                        }
                    },
                    Err(err) => kprintln!("swap failed: {:?}", err),
                }
            }
            Err(err) => {
                if dry_run {
                    kprintln!("dry-run failed: {:?}", err);
                } else {
                    kprintln!("plug failed: {:?}", err);
                }
            }
        }
    }

    fn unplug_slot(&mut self, slot: &str) {
        match self.board.unplug(slot) {
            Ok(Some(provider)) => kprintln!("unplugged {} from {}", slot, provider),
            Ok(None) => kprintln!("slot already empty: {}", slot),
            Err(BoardError::SlotNotFound) => kprintln!("slot not found: {}", slot),
            Err(BoardError::InvalidSlot) => kprintln!("invalid slot: {}", slot),
            Err(err) => kprintln!("unplug failed: {:?}", err),
        }
    }

    fn print_graph(&self) {
        let mut rows = Vec::new();
        for module in &self.modules {
            let Some(manifest) = &module.manifest else {
                continue;
            };
            let state = if module.running {
                "running"
            } else {
                "installed"
            };
            rows.push(GraphRow {
                name: module.name.clone(),
                state: state.to_string(),
                depends: manifest.depends.clone(),
            });
        }
        for entry in &self.catalog {
            let state = if entry.verified { "catalog" } else { "unsigned" };
            rows.push(GraphRow {
                name: entry.name.clone(),
                state: state.to_string(),
                depends: entry.manifest.depends.clone(),
            });
        }
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        kprintln!("{}", format_graph(&rows));
    }

    fn print_sysinfo(&self) {
        let gpu_devices = self
            .board
            .provider_for("ruzzle.slot.gpu@1")
            .map(|_| 1)
            .unwrap_or(0);
        let metrics = SystemMetrics {
            cpu_total: smp::cpu_total(),
            cpu_online: smp::cpu_online(),
            gpu_devices,
        };
        let info = build_system_info(&self.settings, &self.session, &self.board, metrics);
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

fn normalize_slot_filter(slot: &str) -> Result<String, ()> {
    let trimmed = slot.trim();
    if trimmed.is_empty() {
        return Err(());
    }
    if let Some((base, version)) = trimmed.rsplit_once('@') {
        if base.is_empty()
            || version.is_empty()
            || !version.chars().all(|ch| ch.is_ascii_digit())
        {
            return Err(());
        }
        return Ok(trimmed.to_string());
    }
    Ok(format!("{}@1", trimmed))
}

fn default_mounts() -> Vec<MountEntry> {
    vec![MountEntry {
        source: "memfs".to_string(),
        target: "/".to_string(),
        fstype: "memfs".to_string(),
    }]
}

fn join_path(base: &str, child: &str) -> String {
    if base == "/" {
        format!("/{}", child)
    } else {
        format!("{}/{}", base, child)
    }
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
                    verified: bundle.verified,
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
            verified: true,
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
            verified: true,
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
        PuzzleSlot::new("ruzzle.slot.console@1", true),
        PuzzleSlot::new("ruzzle.slot.shell@1", true),
        PuzzleSlot::new("ruzzle.slot.fs@1", true),
        PuzzleSlot::new("ruzzle.slot.user@1", true),
        PuzzleSlot::new("ruzzle.slot.settings@1", true),
        PuzzleSlot::new("ruzzle.slot.session@1", true),
        PuzzleSlot::new("ruzzle.slot.setup@1", false),
        PuzzleSlot::new("ruzzle.slot.net@1", false),
        PuzzleSlot::new("ruzzle.slot.netmgr@1", false),
        PuzzleSlot::new("ruzzle.slot.input@1", false),
        PuzzleSlot::new("ruzzle.slot.device@1", false),
        PuzzleSlot::new("ruzzle.slot.editor@1", false),
        PuzzleSlot::new("ruzzle.slot.filemgr@1", false),
        PuzzleSlot::new("ruzzle.slot.sysinfo@1", false),
        PuzzleSlot::new("ruzzle.slot.toolchain@1", false),
        PuzzleSlot::new("ruzzle.slot.container@1", false),
        PuzzleSlot::new("ruzzle.slot.server@1", false),
        PuzzleSlot::new("ruzzle.slot.gpu@1", false),
        PuzzleSlot::new("ruzzle.slot.ml@1", false),
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
