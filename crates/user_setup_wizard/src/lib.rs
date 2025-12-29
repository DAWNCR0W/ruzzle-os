#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use user_fs_service::{FileSystem, FsError};
use user_settings_service::{SettingsError, SystemSettings};
use user_user_service::{default_home_dir, is_valid_user_name, UserError, UserManager};

#[cfg(test)]
use core::cell::Cell;

const BASE_DIRECTORIES: [&str; 11] = [
    "/system",
    "/etc",
    "/var",
    "/tmp",
    "/bin",
    "/usr",
    "/home",
    "/srv",
    "/opt",
    "/lib",
    "/dev",
];
const SYSTEM_DIRECTORIES: [&str; 4] = ["/system/bin", "/system/lib", "/system/modules", "/system/config"];
const VAR_DIRECTORIES: [&str; 3] = ["/var/log", "/var/tmp", "/var/run"];
const USR_DIRECTORIES: [&str; 2] = ["/usr/bin", "/usr/lib"];

#[cfg(test)]
thread_local! {
    static BASE_DIR_OVERRIDE: Cell<Option<&'static [&'static str]>> = Cell::new(None);
}

/// Input collected during first boot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupPlan {
    pub username: String,
    pub is_admin: bool,
    pub hostname: String,
    pub locale: String,
    pub timezone: String,
    pub keyboard: String,
}

impl SetupPlan {
    /// Creates a plan with explicit values.
    pub fn new(
        username: &str,
        is_admin: bool,
        hostname: &str,
        locale: &str,
        timezone: &str,
        keyboard: &str,
    ) -> Self {
        Self {
            username: username.to_string(),
            is_admin,
            hostname: hostname.to_string(),
            locale: locale.to_string(),
            timezone: timezone.to_string(),
            keyboard: keyboard.to_string(),
        }
    }
}

/// Captures the work performed by the wizard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapReport {
    pub user: String,
    pub created_dirs: Vec<String>,
    pub created_files: Vec<String>,
}

/// Errors returned by the wizard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupError {
    InvalidUser,
    User(UserError),
    Fs(FsError),
    Settings(SettingsError),
}

/// Runs the first-boot setup wizard.
pub fn run_first_boot(
    fs: &mut FileSystem,
    users: &mut UserManager,
    settings: &mut SystemSettings,
    plan: &SetupPlan,
) -> Result<BootstrapReport, SetupError> {
    if !is_valid_user_name(&plan.username) {
        return Err(SetupError::InvalidUser);
    }

    settings
        .set_hostname(&plan.hostname)
        .map_err(SetupError::Settings)?;
    settings
        .set_locale(&plan.locale)
        .map_err(SetupError::Settings)?;
    settings
        .set_timezone(&plan.timezone)
        .map_err(SetupError::Settings)?;
    settings
        .set_keyboard(&plan.keyboard)
        .map_err(SetupError::Settings)?;

    let mut report = BootstrapReport {
        user: plan.username.clone(),
        created_dirs: Vec::new(),
        created_files: Vec::new(),
    };

    for dir in base_directories() {
        ensure_dir(fs, dir, &mut report)?;
    }
    for dir in system_directories() {
        ensure_dir(fs, dir, &mut report)?;
    }
    for dir in var_directories() {
        ensure_dir(fs, dir, &mut report)?;
    }
    for dir in usr_directories() {
        ensure_dir(fs, dir, &mut report)?;
    }

    let home = default_home_dir(&plan.username);
    ensure_dir(fs, &home, &mut report)?;
    for suffix in ["docs", "bin", ".config", "downloads"].iter() {
        let path = format!("{}/{}", home, suffix);
        ensure_dir(fs, &path, &mut report)?;
    }

    write_file(fs, "/etc/hostname", settings.hostname(), &mut report)?;
    write_file(fs, "/etc/locale", settings.locale(), &mut report)?;
    write_file(fs, "/etc/timezone", settings.timezone(), &mut report)?;
    write_file(fs, "/etc/keyboard", settings.keyboard(), &mut report)?;
    write_file(fs, "/etc/ruzzle.conf", &settings.to_config_text(), &mut report)?;

    users
        .add_user(&plan.username, plan.is_admin)
        .map_err(SetupError::User)?;

    Ok(report)
}

fn base_directories() -> &'static [&'static str] {
    #[cfg(test)]
    if let Some(override_dirs) = BASE_DIR_OVERRIDE.with(|cell| cell.get()) {
        return override_dirs;
    }
    &BASE_DIRECTORIES
}

fn system_directories() -> &'static [&'static str] {
    &SYSTEM_DIRECTORIES
}

fn var_directories() -> &'static [&'static str] {
    &VAR_DIRECTORIES
}

fn usr_directories() -> &'static [&'static str] {
    &USR_DIRECTORIES
}

fn ensure_dir(
    fs: &mut FileSystem,
    path: &str,
    report: &mut BootstrapReport,
) -> Result<(), SetupError> {
    match fs.mkdir(path) {
        Ok(()) => {
            report.created_dirs.push(path.to_string());
            Ok(())
        }
        Err(FsError::AlreadyExists) => Ok(()),
        Err(err) => Err(SetupError::Fs(err)),
    }
}

fn write_file(
    fs: &mut FileSystem,
    path: &str,
    contents: &str,
    report: &mut BootstrapReport,
) -> Result<(), SetupError> {
    fs.write_file(path, contents.as_bytes())
        .map_err(SetupError::Fs)?;
    report.created_files.push(path.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_base_override(dirs: Option<&'static [&'static str]>) {
        BASE_DIR_OVERRIDE.with(|cell| cell.set(dirs));
    }

    fn plan() -> SetupPlan {
        SetupPlan::new(
            "root",
            true,
            "ruzzle",
            "en_US.UTF-8",
            "UTC",
            "us",
        )
    }

    #[test]
    fn run_first_boot_creates_dirs_and_user() {
        let mut fs = FileSystem::new();
        let mut users = UserManager::new();
        let mut settings = SystemSettings::new_defaults();

        let report = run_first_boot(&mut fs, &mut users, &mut settings, &plan()).unwrap();
        assert_eq!(report.user, "root");
        assert!(users.has_user("root"));
        assert!(fs.list_dir("/home").unwrap().contains(&"root".to_string()));
        assert!(fs.list_dir("/system").unwrap().contains(&"bin".to_string()));
        assert_eq!(fs.read_file("/etc/hostname").unwrap(), b"ruzzle");
        assert_eq!(fs.read_file("/etc/ruzzle.conf").unwrap().len() > 0, true);
    }

    #[test]
    fn run_first_boot_rejects_invalid_user() {
        let mut fs = FileSystem::new();
        let mut users = UserManager::new();
        let mut settings = SystemSettings::new_defaults();
        let mut plan = plan();
        plan.username = "Bad User".to_string();
        assert_eq!(
            run_first_boot(&mut fs, &mut users, &mut settings, &plan),
            Err(SetupError::InvalidUser)
        );
    }

    #[test]
    fn run_first_boot_rejects_invalid_settings() {
        let mut fs = FileSystem::new();
        let mut users = UserManager::new();
        let mut settings = SystemSettings::new_defaults();
        let mut plan = plan();
        plan.hostname = "Bad Host".to_string();
        assert_eq!(
            run_first_boot(&mut fs, &mut users, &mut settings, &plan),
            Err(SetupError::Settings(SettingsError::InvalidHostname))
        );
    }

    #[test]
    fn run_first_boot_rejects_invalid_locale() {
        let mut fs = FileSystem::new();
        let mut users = UserManager::new();
        let mut settings = SystemSettings::new_defaults();
        let mut plan = plan();
        plan.locale = "bad locale".to_string();
        assert_eq!(
            run_first_boot(&mut fs, &mut users, &mut settings, &plan),
            Err(SetupError::Settings(SettingsError::InvalidLocale))
        );
    }

    #[test]
    fn run_first_boot_rejects_invalid_timezone() {
        let mut fs = FileSystem::new();
        let mut users = UserManager::new();
        let mut settings = SystemSettings::new_defaults();
        let mut plan = plan();
        plan.timezone = "/bad".to_string();
        assert_eq!(
            run_first_boot(&mut fs, &mut users, &mut settings, &plan),
            Err(SetupError::Settings(SettingsError::InvalidTimezone))
        );
    }

    #[test]
    fn run_first_boot_rejects_invalid_keyboard() {
        let mut fs = FileSystem::new();
        let mut users = UserManager::new();
        let mut settings = SystemSettings::new_defaults();
        let mut plan = plan();
        plan.keyboard = "bad layout".to_string();
        assert_eq!(
            run_first_boot(&mut fs, &mut users, &mut settings, &plan),
            Err(SetupError::Settings(SettingsError::InvalidKeyboard))
        );
    }

    #[test]
    fn run_first_boot_allows_existing_dirs() {
        let mut fs = FileSystem::new();
        fs.mkdir("/etc").unwrap();
        fs.mkdir("/home").unwrap();
        let mut users = UserManager::new();
        let mut settings = SystemSettings::new_defaults();

        let report = run_first_boot(&mut fs, &mut users, &mut settings, &plan()).unwrap();
        assert!(report.created_dirs.contains(&"/system".to_string()));
        assert!(fs.list_dir("/etc").is_ok());
    }

    #[test]
    fn run_first_boot_rejects_invalid_base_directory() {
        static BAD_BASE: [&str; 1] = ["/bad//path"];
        set_base_override(Some(&BAD_BASE));

        let mut fs = FileSystem::new();
        let mut users = UserManager::new();
        let mut settings = SystemSettings::new_defaults();
        let result = run_first_boot(&mut fs, &mut users, &mut settings, &plan());

        set_base_override(None);
        assert_eq!(result, Err(SetupError::Fs(FsError::InvalidPath)));
    }

    #[test]
    fn run_first_boot_rejects_system_directory_when_parent_is_file() {
        let mut fs = FileSystem::new();
        fs.write_file("/system", b"x").unwrap();
        let mut users = UserManager::new();
        let mut settings = SystemSettings::new_defaults();
        let result = run_first_boot(&mut fs, &mut users, &mut settings, &plan());
        assert_eq!(result, Err(SetupError::Fs(FsError::NotDir)));
    }

    #[test]
    fn run_first_boot_rejects_var_directory_when_parent_is_file() {
        let mut fs = FileSystem::new();
        fs.write_file("/var", b"x").unwrap();
        let mut users = UserManager::new();
        let mut settings = SystemSettings::new_defaults();
        let result = run_first_boot(&mut fs, &mut users, &mut settings, &plan());
        assert_eq!(result, Err(SetupError::Fs(FsError::NotDir)));
    }

    #[test]
    fn run_first_boot_rejects_usr_directory_when_parent_is_file() {
        let mut fs = FileSystem::new();
        fs.write_file("/usr", b"x").unwrap();
        let mut users = UserManager::new();
        let mut settings = SystemSettings::new_defaults();
        let result = run_first_boot(&mut fs, &mut users, &mut settings, &plan());
        assert_eq!(result, Err(SetupError::Fs(FsError::NotDir)));
    }

    #[test]
    fn run_first_boot_rejects_home_subdir_when_parent_is_file() {
        let mut fs = FileSystem::new();
        fs.mkdir("/home").unwrap();
        fs.write_file("/home/root", b"x").unwrap();
        let mut users = UserManager::new();
        let mut settings = SystemSettings::new_defaults();
        let result = run_first_boot(&mut fs, &mut users, &mut settings, &plan());
        assert_eq!(result, Err(SetupError::Fs(FsError::NotDir)));
    }

    #[test]
    fn run_first_boot_propagates_write_file_errors() {
        let targets = [
            "/etc/hostname",
            "/etc/locale",
            "/etc/timezone",
            "/etc/keyboard",
            "/etc/ruzzle.conf",
        ];
        for target in targets {
            let mut fs = FileSystem::new();
            fs.mkdir("/etc").unwrap();
            fs.mkdir(target).unwrap();
            let mut users = UserManager::new();
            let mut settings = SystemSettings::new_defaults();
            let result = run_first_boot(&mut fs, &mut users, &mut settings, &plan());
            assert_eq!(result, Err(SetupError::Fs(FsError::IsDir)));
        }
    }

    #[test]
    fn run_first_boot_rejects_duplicate_user() {
        let mut fs = FileSystem::new();
        let mut users = UserManager::new();
        users.add_user("root", true).unwrap();
        let mut settings = SystemSettings::new_defaults();
        let result = run_first_boot(&mut fs, &mut users, &mut settings, &plan());
        assert_eq!(result, Err(SetupError::User(UserError::AlreadyExists)));
    }

    #[test]
    fn run_first_boot_propagates_fs_errors() {
        let mut fs = FileSystem::new();
        let mut users = UserManager::new();
        let mut settings = SystemSettings::new_defaults();
        let mut plan = plan();
        plan.username = "root".to_string();
        fs.write_file("/home", b"x").unwrap();
        let result = run_first_boot(&mut fs, &mut users, &mut settings, &plan);
        assert_eq!(result, Err(SetupError::Fs(FsError::NotDir)));
    }
}
